// GENERATED CODE - DO NOT MODIFY BY HAND

import 'dart:async';
import 'dart:typed_data';

import '../../link/CoreLinkProtocol.dart';

class CoreProxyErrorDetails {
  const CoreProxyErrorDetails({
    required this.errorType,
    required this.message,
    this.variant,
    this.kind,
    this.httpStatus,
    this.remoteMessage,
    this.fields = const <String, Object?>{},
  });

  factory CoreProxyErrorDetails.fromCoreLinkError(CoreLinkError error) {
    final details = error.details;
    if (details is Map<String, Object?>) {
      return CoreProxyErrorDetails.fromJson(details, message: error.message);
    }
    return CoreProxyErrorDetails(errorType: error.code, message: error.message);
  }

  factory CoreProxyErrorDetails.fromJson(Map<String, Object?> json, {String? message}) {
    final classification = json['classification'];
    final fields = json['fields'];
    return CoreProxyErrorDetails(
      errorType: _stringValue(json['errorType']) ?? 'unknown',
      message: _stringValue(json['message']) ?? message ?? '',
      variant: _stringValue(json['variant']),
      kind: classification is Map<String, Object?> ? _stringValue(classification['kind']) : _stringValue(json['kind']),
      httpStatus: _intValue(json['httpStatus']),
      remoteMessage: _stringValue(json['remoteMessage']),
      fields: fields is Map<String, Object?> ? fields : const <String, Object?>{},
    );
  }

  final String errorType;
  final String message;
  final String? variant;
  final String? kind;
  final int? httpStatus;
  final String? remoteMessage;
  final Map<String, Object?> fields;

  String? stringField(String name) => _stringValue(fields[name]);

  static String? _stringValue(Object? value) {
    if (value is String && value.trim().isNotEmpty) {
      return value.trim();
    }
    return null;
  }

  static int? _intValue(Object? value) {
    if (value is int) {
      return value;
    }
    return null;
  }
}

/// Generated Dart model for Rust type `operit_core_proxy::RuntimeRemoteLinkService::RuntimeRemoteDiscoveredDevice`.
class RuntimeRemoteDiscoveredDevice {
  const RuntimeRemoteDiscoveredDevice({
    required this.deviceId,
    required this.displayName,
    required this.platform,
    required this.model,
    required this.baseUrl,
    required this.hostname,
    required this.port,
    required this.tokenHash,
    required this.version,
  });

  factory RuntimeRemoteDiscoveredDevice.fromJson(Map<String, Object?> json) {
    return RuntimeRemoteDiscoveredDevice(
      deviceId: json['deviceId'] as String,
      displayName: json['displayName'] as String,
      platform: json['platform'] as String,
      model: json['model'] as String,
      baseUrl: json['baseUrl'] as String,
      hostname: json['hostname'] as String,
      port: json['port'] as int,
      tokenHash: json['tokenHash'] as String,
      version: json['version'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'deviceId': deviceId,
      'displayName': displayName,
      'platform': platform,
      'model': model,
      'baseUrl': baseUrl,
      'hostname': hostname,
      'port': port,
      'tokenHash': tokenHash,
      'version': version,
    };
  }

  /// Rust field `deviceId` serialized as `deviceId`.
  final String deviceId;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `platform` serialized as `platform`.
  final String platform;
  /// Rust field `model` serialized as `model`.
  final String model;
  /// Rust field `baseUrl` serialized as `baseUrl`.
  final String baseUrl;
  /// Rust field `hostname` serialized as `hostname`.
  final String hostname;
  /// Rust field `port` serialized as `port`.
  final int port;
  /// Rust field `tokenHash` serialized as `tokenHash`.
  final String tokenHash;
  /// Rust field `version` serialized as `version`.
  final String version;
}

/// Generated Dart model for Rust type `operit_core_proxy::RuntimeRemoteLinkService::RuntimeRemotePairStartResult`.
class RuntimeRemotePairStartResult {
  const RuntimeRemotePairStartResult({
    required this.pairingId,
    required this.pairingServiceVersion,
    required this.coreDeviceId,
    required this.coreDeviceInfo,
  });

  factory RuntimeRemotePairStartResult.fromJson(Map<String, Object?> json) {
    return RuntimeRemotePairStartResult(
      pairingId: json['pairingId'] as String,
      pairingServiceVersion: json['pairingServiceVersion'] as int,
      coreDeviceId: json['coreDeviceId'] as String,
      coreDeviceInfo: RemoteDeviceInfo.fromJson(json['coreDeviceInfo'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'pairingId': pairingId,
      'pairingServiceVersion': pairingServiceVersion,
      'coreDeviceId': coreDeviceId,
      'coreDeviceInfo': coreDeviceInfo.toJson(),
    };
  }

  /// Rust field `pairingId` serialized as `pairingId`.
  final String pairingId;
  /// Rust field `pairingServiceVersion` serialized as `pairingServiceVersion`.
  final int pairingServiceVersion;
  /// Rust field `coreDeviceId` serialized as `coreDeviceId`.
  final String coreDeviceId;
  /// Rust field `coreDeviceInfo` serialized as `coreDeviceInfo`.
  final RemoteDeviceInfo coreDeviceInfo;
}

/// Generated Dart model for Rust type `operit_core_proxy::RuntimeRemoteLinkService::RuntimeRemoteProbeResult`.
class RuntimeRemoteProbeResult {
  const RuntimeRemoteProbeResult({
    required this.coreDeviceId,
  });

  factory RuntimeRemoteProbeResult.fromJson(Map<String, Object?> json) {
    return RuntimeRemoteProbeResult(
      coreDeviceId: json['coreDeviceId'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'coreDeviceId': coreDeviceId,
    };
  }

  /// Rust field `coreDeviceId` serialized as `coreDeviceId`.
  final String coreDeviceId;
}

/// Generated Dart model for Rust type `operit_core_proxy::RuntimeRemoteLinkService::RuntimeRemoteSyncResult`.
class RuntimeRemoteSyncResult {
  const RuntimeRemoteSyncResult({
    required this.rounds,
    required this.localApplied,
    required this.remoteApplied,
  });

  factory RuntimeRemoteSyncResult.fromJson(Map<String, Object?> json) {
    return RuntimeRemoteSyncResult(
      rounds: json['rounds'] as int,
      localApplied: json['localApplied'] as int,
      remoteApplied: json['remoteApplied'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'rounds': rounds,
      'localApplied': localApplied,
      'remoteApplied': remoteApplied,
    };
  }

  /// Rust field `rounds` serialized as `rounds`.
  final int rounds;
  /// Rust field `localApplied` serialized as `localApplied`.
  final int localApplied;
  /// Rust field `remoteApplied` serialized as `remoteApplied`.
  final int remoteApplied;
}

/// Generated Dart enum for Rust type `operit_host_api::CapabilityOperation`.
enum CapabilityOperation {
  read('Read'),
  write('Write'),
  execute('Execute'),
  connect('Connect'),
  ;

  const CapabilityOperation(this.value);

  final String value;

  factory CapabilityOperation.fromJson(Object? value) {
    return switch (value) {
      'Read' => CapabilityOperation.read,
      'Write' => CapabilityOperation.write,
      'Execute' => CapabilityOperation.execute,
      'Connect' => CapabilityOperation.connect,
      _ => throw ArgumentError('Unknown CapabilityOperation: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_host_api::CapabilityScope`.
enum CapabilityScope {
  fileSystem('FileSystem'),
  system('System'),
  network('Network'),
  runtime('Runtime'),
  device('Device'),
  media('Media'),
  ;

  const CapabilityScope(this.value);

  final String value;

  factory CapabilityScope.fromJson(Object? value) {
    return switch (value) {
      'FileSystem' => CapabilityScope.fileSystem,
      'System' => CapabilityScope.system,
      'Network' => CapabilityScope.network,
      'Runtime' => CapabilityScope.runtime,
      'Device' => CapabilityScope.device,
      'Media' => CapabilityScope.media,
      _ => throw ArgumentError('Unknown CapabilityScope: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_host_api::HostCapability`.
class HostCapability {
  const HostCapability({
    required this.id,
    required this.displayName,
    required this.scope,
    required this.operations,
  });

  factory HostCapability.fromJson(Map<String, Object?> json) {
    return HostCapability(
      id: json['id'] as String,
      displayName: json['displayName'] as String,
      scope: CapabilityScope.fromJson(json['scope']),
      operations: (json['operations'] as List<Object?>).map((item) => CapabilityOperation.fromJson(item)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'displayName': displayName,
      'scope': scope.toJson(),
      'operations': operations.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `scope` serialized as `scope`.
  final CapabilityScope scope;
  /// Rust field `operations` serialized as `operations`.
  final List<CapabilityOperation> operations;
}

/// Generated Dart model for Rust type `operit_host_api::HostEnvironmentDescriptor`.
class HostEnvironmentDescriptor {
  const HostEnvironmentDescriptor({
    required this.id,
    required this.displayName,
    required this.platform,
    required this.privilege,
    required this.isolation,
    required this.pathStyleDescriptionEn,
    required this.pathStyleDescriptionCn,
    required this.examplePaths,
    required this.usesEnvironmentParameter,
    required this.environmentParameterDescriptionEn,
    required this.environmentParameterDescriptionCn,
    required this.capabilities,
    required this.structuredCapabilities,
    required this.onboardingRequirements,
    required this.workspaceRoots,
  });

  factory HostEnvironmentDescriptor.fromJson(Map<String, Object?> json) {
    return HostEnvironmentDescriptor(
      id: json['id'] as String,
      displayName: json['displayName'] as String,
      platform: HostPlatform.fromJson(json['platform']),
      privilege: HostPrivilege.fromJson(json['privilege']),
      isolation: HostIsolation.fromJson(json['isolation']),
      pathStyleDescriptionEn: json['pathStyleDescriptionEn'] as String,
      pathStyleDescriptionCn: json['pathStyleDescriptionCn'] as String,
      examplePaths: (json['examplePaths'] as List<Object?>).map((item) => item as String).toList(growable: false),
      usesEnvironmentParameter: json['usesEnvironmentParameter'] as bool,
      environmentParameterDescriptionEn: json['environmentParameterDescriptionEn'] as String,
      environmentParameterDescriptionCn: json['environmentParameterDescriptionCn'] as String,
      capabilities: (json['capabilities'] as List<Object?>).map((item) => item as String).toList(growable: false),
      structuredCapabilities: (json['structuredCapabilities'] as List<Object?>).map((item) => HostCapability.fromJson(item as Map<String, Object?>)).toList(growable: false),
      onboardingRequirements: (json['onboardingRequirements'] as List<Object?>).map((item) => HostOnboardingRequirement.fromJson(item as Map<String, Object?>)).toList(growable: false),
      workspaceRoots: (json['workspaceRoots'] as List<Object?>).map((item) => WorkspaceRootDescriptor.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'displayName': displayName,
      'platform': platform.toJson(),
      'privilege': privilege.toJson(),
      'isolation': isolation.toJson(),
      'pathStyleDescriptionEn': pathStyleDescriptionEn,
      'pathStyleDescriptionCn': pathStyleDescriptionCn,
      'examplePaths': examplePaths.map((item) => item).toList(growable: false),
      'usesEnvironmentParameter': usesEnvironmentParameter,
      'environmentParameterDescriptionEn': environmentParameterDescriptionEn,
      'environmentParameterDescriptionCn': environmentParameterDescriptionCn,
      'capabilities': capabilities.map((item) => item).toList(growable: false),
      'structuredCapabilities': structuredCapabilities.map((item) => item.toJson()).toList(growable: false),
      'onboardingRequirements': onboardingRequirements.map((item) => item.toJson()).toList(growable: false),
      'workspaceRoots': workspaceRoots.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `platform` serialized as `platform`.
  final HostPlatform platform;
  /// Rust field `privilege` serialized as `privilege`.
  final HostPrivilege privilege;
  /// Rust field `isolation` serialized as `isolation`.
  final HostIsolation isolation;
  /// Rust field `pathStyleDescriptionEn` serialized as `pathStyleDescriptionEn`.
  final String pathStyleDescriptionEn;
  /// Rust field `pathStyleDescriptionCn` serialized as `pathStyleDescriptionCn`.
  final String pathStyleDescriptionCn;
  /// Rust field `examplePaths` serialized as `examplePaths`.
  final List<String> examplePaths;
  /// Rust field `usesEnvironmentParameter` serialized as `usesEnvironmentParameter`.
  final bool usesEnvironmentParameter;
  /// Rust field `environmentParameterDescriptionEn` serialized as `environmentParameterDescriptionEn`.
  final String environmentParameterDescriptionEn;
  /// Rust field `environmentParameterDescriptionCn` serialized as `environmentParameterDescriptionCn`.
  final String environmentParameterDescriptionCn;
  /// Rust field `capabilities` serialized as `capabilities`.
  final List<String> capabilities;
  /// Rust field `structuredCapabilities` serialized as `structuredCapabilities`.
  final List<HostCapability> structuredCapabilities;
  /// Rust field `onboardingRequirements` serialized as `onboardingRequirements`.
  final List<HostOnboardingRequirement> onboardingRequirements;
  /// Rust field `workspaceRoots` serialized as `workspaceRoots`.
  final List<WorkspaceRootDescriptor> workspaceRoots;
}

/// Generated Dart enum for Rust type `operit_host_api::HostIsolation`.
enum HostIsolation {
  none('None'),
  osAppSandbox('OsAppSandbox'),
  container('Container'),
  virtualMachine('VirtualMachine'),
  ;

  const HostIsolation(this.value);

  final String value;

  factory HostIsolation.fromJson(Object? value) {
    return switch (value) {
      'None' => HostIsolation.none,
      'OsAppSandbox' => HostIsolation.osAppSandbox,
      'Container' => HostIsolation.container,
      'VirtualMachine' => HostIsolation.virtualMachine,
      _ => throw ArgumentError('Unknown HostIsolation: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_host_api::HostOnboardingRequirement`.
class HostOnboardingRequirement {
  const HostOnboardingRequirement({
    required this.id,
    required this.title,
    required this.description,
    required this.capabilityIds,
    required this.status,
    required this.action,
  });

  factory HostOnboardingRequirement.fromJson(Map<String, Object?> json) {
    return HostOnboardingRequirement(
      id: json['id'] as String,
      title: json['title'] as String,
      description: json['description'] as String,
      capabilityIds: (json['capabilityIds'] as List<Object?>).map((item) => item as String).toList(growable: false),
      status: HostRequirementStatus.fromJson(json['status']),
      action: HostRequirementAction.fromJson(json['action']),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'title': title,
      'description': description,
      'capabilityIds': capabilityIds.map((item) => item).toList(growable: false),
      'status': status.toJson(),
      'action': action.toJson(),
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `capabilityIds` serialized as `capabilityIds`.
  final List<String> capabilityIds;
  /// Rust field `status` serialized as `status`.
  final HostRequirementStatus status;
  /// Rust field `action` serialized as `action`.
  final HostRequirementAction action;
}

/// Generated Dart enum for Rust type `operit_host_api::HostPlatform`.
enum HostPlatform {
  android('Android'),
  ohos('Ohos'),
  windows('Windows'),
  linux('Linux'),
  macos('Macos'),
  ios('Ios'),
  web('Web'),
  other('Other'),
  ;

  const HostPlatform(this.value);

  final String value;

  factory HostPlatform.fromJson(Object? value) {
    return switch (value) {
      'Android' => HostPlatform.android,
      'Ohos' => HostPlatform.ohos,
      'Windows' => HostPlatform.windows,
      'Linux' => HostPlatform.linux,
      'Macos' => HostPlatform.macos,
      'Ios' => HostPlatform.ios,
      'Web' => HostPlatform.web,
      'Other' => HostPlatform.other,
      _ => throw ArgumentError('Unknown HostPlatform: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_host_api::HostPrivilege`.
enum HostPrivilege {
  normal('Normal'),
  androidShizuku('AndroidShizuku'),
  androidRoot('AndroidRoot'),
  administrator('Administrator'),
  root('Root'),
  serviceAccount('ServiceAccount'),
  ;

  const HostPrivilege(this.value);

  final String value;

  factory HostPrivilege.fromJson(Object? value) {
    return switch (value) {
      'Normal' => HostPrivilege.normal,
      'AndroidShizuku' => HostPrivilege.androidShizuku,
      'AndroidRoot' => HostPrivilege.androidRoot,
      'Administrator' => HostPrivilege.administrator,
      'Root' => HostPrivilege.root,
      'ServiceAccount' => HostPrivilege.serviceAccount,
      _ => throw ArgumentError('Unknown HostPrivilege: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_host_api::HostRequirementAction`.
enum HostRequirementAction {
  runtimePermission('RuntimePermission'),
  openSystemSettings('OpenSystemSettings'),
  hostManaged('HostManaged'),
  none('None'),
  ;

  const HostRequirementAction(this.value);

  final String value;

  factory HostRequirementAction.fromJson(Object? value) {
    return switch (value) {
      'RuntimePermission' => HostRequirementAction.runtimePermission,
      'OpenSystemSettings' => HostRequirementAction.openSystemSettings,
      'HostManaged' => HostRequirementAction.hostManaged,
      'None' => HostRequirementAction.none,
      _ => throw ArgumentError('Unknown HostRequirementAction: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_host_api::HostRequirementStatus`.
enum HostRequirementStatus {
  satisfied('Satisfied'),
  missing('Missing'),
  unavailable('Unavailable'),
  ;

  const HostRequirementStatus(this.value);

  final String value;

  factory HostRequirementStatus.fromJson(Object? value) {
    return switch (value) {
      'Satisfied' => HostRequirementStatus.satisfied,
      'Missing' => HostRequirementStatus.missing,
      'Unavailable' => HostRequirementStatus.unavailable,
      _ => throw ArgumentError('Unknown HostRequirementStatus: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_host_api::WorkspaceRootDescriptor`.
class WorkspaceRootDescriptor {
  const WorkspaceRootDescriptor({
    required this.id,
    required this.displayName,
    required this.vfsRoot,
    required this.physicalRoot,
  });

  factory WorkspaceRootDescriptor.fromJson(Map<String, Object?> json) {
    return WorkspaceRootDescriptor(
      id: json['id'] as String,
      displayName: json['displayName'] as String,
      vfsRoot: json['vfsRoot'] as String,
      physicalRoot: json['physicalRoot'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'displayName': displayName,
      'vfsRoot': vfsRoot,
      'physicalRoot': physicalRoot,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `vfsRoot` serialized as `vfsRoot`.
  final String vfsRoot;
  /// Rust field `physicalRoot` serialized as `physicalRoot`.
  final String physicalRoot;
}

/// Generated Dart model for Rust type `operit_link_access::AcceptedRemoteSessionRecord`.
class AcceptedRemoteSessionRecord {
  const AcceptedRemoteSessionRecord({
    required this.deviceId,
    required this.deviceInfo,
    required this.pairingServiceVersion,
    required this.sessionSecret,
  });

  factory AcceptedRemoteSessionRecord.fromJson(Map<String, Object?> json) {
    return AcceptedRemoteSessionRecord(
      deviceId: json['deviceId'] as String,
      deviceInfo: RemoteDeviceInfo.fromJson(json['deviceInfo'] as Map<String, Object?>),
      pairingServiceVersion: json['pairingServiceVersion'] as int,
      sessionSecret: json['sessionSecret'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'deviceId': deviceId,
      'deviceInfo': deviceInfo.toJson(),
      'pairingServiceVersion': pairingServiceVersion,
      'sessionSecret': sessionSecret,
    };
  }

  /// Rust field `deviceId` serialized as `deviceId`.
  final String deviceId;
  /// Rust field `deviceInfo` serialized as `deviceInfo`.
  final RemoteDeviceInfo deviceInfo;
  /// Rust field `pairingServiceVersion` serialized as `pairingServiceVersion`.
  final int pairingServiceVersion;
  /// Rust field `sessionSecret` serialized as `sessionSecret`.
  final String sessionSecret;
}

/// Generated Dart model for Rust type `operit_link_access::LinkAccessAutoSyncConfig`.
class LinkAccessAutoSyncConfig {
  const LinkAccessAutoSyncConfig({
    required this.autoSyncRemoteNames,
    required this.updatedAt,
  });

  factory LinkAccessAutoSyncConfig.fromJson(Map<String, Object?> json) {
    return LinkAccessAutoSyncConfig(
      autoSyncRemoteNames: (json['autoSyncRemoteNames'] as List<Object?>).map((item) => item as String).toList(growable: false),
      updatedAt: json['updatedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'autoSyncRemoteNames': autoSyncRemoteNames.map((item) => item).toList(growable: false),
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `autoSyncRemoteNames` serialized as `autoSyncRemoteNames`.
  final List<String> autoSyncRemoteNames;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
}

/// Generated Dart model for Rust type `operit_link_access::LinkAccessHostConfig`.
class LinkAccessHostConfig {
  const LinkAccessHostConfig({
    required this.bindAddress,
    required this.token,
    required this.webAccessEnabled,
    required this.discoveryEnabled,
    required this.portMode,
    required this.updatedAt,
  });

  factory LinkAccessHostConfig.fromJson(Map<String, Object?> json) {
    return LinkAccessHostConfig(
      bindAddress: json['bindAddress'] as String,
      token: json['token'] as String,
      webAccessEnabled: json['webAccessEnabled'] as bool,
      discoveryEnabled: json['discoveryEnabled'] as bool,
      portMode: LinkAccessHostPortMode.fromJson(json['portMode']),
      updatedAt: json['updatedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'bindAddress': bindAddress,
      'token': token,
      'webAccessEnabled': webAccessEnabled,
      'discoveryEnabled': discoveryEnabled,
      'portMode': portMode.toJson(),
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `bindAddress` serialized as `bindAddress`.
  final String bindAddress;
  /// Rust field `token` serialized as `token`.
  final String token;
  /// Rust field `webAccessEnabled` serialized as `webAccessEnabled`.
  final bool webAccessEnabled;
  /// Rust field `discoveryEnabled` serialized as `discoveryEnabled`.
  final bool discoveryEnabled;
  /// Rust field `portMode` serialized as `portMode`.
  final LinkAccessHostPortMode portMode;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
}

/// Generated Dart enum for Rust type `operit_link_access::LinkAccessHostPortMode`.
enum LinkAccessHostPortMode {
  automatic('automatic'),
  fixed('fixed'),
  ;

  const LinkAccessHostPortMode(this.value);

  final String value;

  factory LinkAccessHostPortMode.fromJson(Object? value) {
    return switch (value) {
      'automatic' => LinkAccessHostPortMode.automatic,
      'fixed' => LinkAccessHostPortMode.fixed,
      _ => throw ArgumentError('Unknown LinkAccessHostPortMode: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_link_access::LinkAccessIdentity`.
class LinkAccessIdentity {
  const LinkAccessIdentity({
    required this.deviceId,
    required this.deviceInfo,
  });

  factory LinkAccessIdentity.fromJson(Map<String, Object?> json) {
    return LinkAccessIdentity(
      deviceId: json['deviceId'] as String,
      deviceInfo: RemoteDeviceInfo.fromJson(json['deviceInfo'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'deviceId': deviceId,
      'deviceInfo': deviceInfo.toJson(),
    };
  }

  /// Rust field `deviceId` serialized as `deviceId`.
  final String deviceId;
  /// Rust field `deviceInfo` serialized as `deviceInfo`.
  final RemoteDeviceInfo deviceInfo;
}

/// Generated Dart tagged enum model for Rust type `operit_link_access::LinkAccessRoute`.
class LinkAccessRoute {
  const LinkAccessRoute._({
    this.sessionName = '',
    required this.tag,
  });

  factory LinkAccessRoute.local() => LinkAccessRoute._(tag: 'local');
  factory LinkAccessRoute.remote({required String sessionName, }) => LinkAccessRoute._(tag: 'remote', sessionName: sessionName);
  final String tag;
  final String sessionName;

  factory LinkAccessRoute.fromJson(Object? json) {
    switch (json) {
      case 'local':
        return LinkAccessRoute.local();
      case Map<String, Object?> map:
        final tag = map.keys.single;
        final data = map[tag] as Map<String, Object?>;
        return switch (tag) {
          'remote' => LinkAccessRoute.remote(sessionName: data['sessionName'] as String, ),
          _ => throw ArgumentError('Unknown LinkAccessRoute tag: $tag'),
        };
      default:
        throw ArgumentError('Unknown LinkAccessRoute representation: $json');
    }
  }

  Object toJson() {
    return switch (tag) {
      'local' => 'local',
      'remote' => <String, Object?>{'remote': <String, Object?>{
        'sessionName': sessionName,
      }},
      _ => throw StateError('Unknown LinkAccessRoute tag: $tag'),
    };
  }
}

/// Generated Dart model for Rust type `operit_link_access::LinkAccessRoutingConfig`.
class LinkAccessRoutingConfig {
  const LinkAccessRoutingConfig({
    required this.route,
    required this.updatedAt,
  });

  factory LinkAccessRoutingConfig.fromJson(Map<String, Object?> json) {
    return LinkAccessRoutingConfig(
      route: LinkAccessRoute.fromJson(json['route']),
      updatedAt: json['updatedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'route': route.toJson(),
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `route` serialized as `route`.
  final LinkAccessRoute route;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
}

/// Generated Dart model for Rust type `operit_link_access::PairStartState`.
class PairStartState {
  const PairStartState({
    required this.pairingId,
    required this.pairingServiceVersion,
    required this.clientDeviceId,
    required this.clientDeviceInfo,
    required this.clientPublicKey,
    required this.coreDeviceId,
    required this.coreDeviceInfo,
    required this.clientNonce,
    required this.serverNonce,
    required this.sharedSecret,
  });

  factory PairStartState.fromJson(Map<String, Object?> json) {
    return PairStartState(
      pairingId: json['pairingId'] as String,
      pairingServiceVersion: json['pairingServiceVersion'] as int,
      clientDeviceId: json['clientDeviceId'] as String,
      clientDeviceInfo: RemoteDeviceInfo.fromJson(json['clientDeviceInfo'] as Map<String, Object?>),
      clientPublicKey: json['clientPublicKey'] as String,
      coreDeviceId: json['coreDeviceId'] as String,
      coreDeviceInfo: RemoteDeviceInfo.fromJson(json['coreDeviceInfo'] as Map<String, Object?>),
      clientNonce: json['clientNonce'] as String,
      serverNonce: json['serverNonce'] as String,
      sharedSecret: json['sharedSecret'] as Uint8List,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'pairingId': pairingId,
      'pairingServiceVersion': pairingServiceVersion,
      'clientDeviceId': clientDeviceId,
      'clientDeviceInfo': clientDeviceInfo.toJson(),
      'clientPublicKey': clientPublicKey,
      'coreDeviceId': coreDeviceId,
      'coreDeviceInfo': coreDeviceInfo.toJson(),
      'clientNonce': clientNonce,
      'serverNonce': serverNonce,
      'sharedSecret': sharedSecret,
    };
  }

  /// Rust field `pairingId` serialized as `pairingId`.
  final String pairingId;
  /// Rust field `pairingServiceVersion` serialized as `pairingServiceVersion`.
  final int pairingServiceVersion;
  /// Rust field `clientDeviceId` serialized as `clientDeviceId`.
  final String clientDeviceId;
  /// Rust field `clientDeviceInfo` serialized as `clientDeviceInfo`.
  final RemoteDeviceInfo clientDeviceInfo;
  /// Rust field `clientPublicKey` serialized as `clientPublicKey`.
  final String clientPublicKey;
  /// Rust field `coreDeviceId` serialized as `coreDeviceId`.
  final String coreDeviceId;
  /// Rust field `coreDeviceInfo` serialized as `coreDeviceInfo`.
  final RemoteDeviceInfo coreDeviceInfo;
  /// Rust field `clientNonce` serialized as `clientNonce`.
  final String clientNonce;
  /// Rust field `serverNonce` serialized as `serverNonce`.
  final String serverNonce;
  /// Rust field `sharedSecret` serialized as `sharedSecret`.
  final Uint8List sharedSecret;
}

/// Generated Dart model for Rust type `operit_link_access::PairedRemoteSessionRecord`.
class PairedRemoteSessionRecord {
  const PairedRemoteSessionRecord({
    required this.baseUrl,
    required this.sessionId,
    required this.deviceId,
    required this.coreDeviceId,
    required this.remoteDeviceInfo,
    required this.pairingServiceVersion,
    required this.sessionSecret,
  });

  factory PairedRemoteSessionRecord.fromJson(Map<String, Object?> json) {
    return PairedRemoteSessionRecord(
      baseUrl: json['baseUrl'] as String,
      sessionId: json['sessionId'] as String,
      deviceId: json['deviceId'] as String,
      coreDeviceId: json['coreDeviceId'] as String,
      remoteDeviceInfo: RemoteDeviceInfo.fromJson(json['remoteDeviceInfo'] as Map<String, Object?>),
      pairingServiceVersion: json['pairingServiceVersion'] as int,
      sessionSecret: json['sessionSecret'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'baseUrl': baseUrl,
      'sessionId': sessionId,
      'deviceId': deviceId,
      'coreDeviceId': coreDeviceId,
      'remoteDeviceInfo': remoteDeviceInfo.toJson(),
      'pairingServiceVersion': pairingServiceVersion,
      'sessionSecret': sessionSecret,
    };
  }

  /// Rust field `baseUrl` serialized as `baseUrl`.
  final String baseUrl;
  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `deviceId` serialized as `deviceId`.
  final String deviceId;
  /// Rust field `coreDeviceId` serialized as `coreDeviceId`.
  final String coreDeviceId;
  /// Rust field `remoteDeviceInfo` serialized as `remoteDeviceInfo`.
  final RemoteDeviceInfo remoteDeviceInfo;
  /// Rust field `pairingServiceVersion` serialized as `pairingServiceVersion`.
  final int pairingServiceVersion;
  /// Rust field `sessionSecret` serialized as `sessionSecret`.
  final String sessionSecret;
}

/// Generated Dart model for Rust type `operit_link_access::PendingOutboundPairingRecord`.
class PendingOutboundPairingRecord {
  const PendingOutboundPairingRecord({
    required this.baseUrl,
    required this.state,
  });

  factory PendingOutboundPairingRecord.fromJson(Map<String, Object?> json) {
    return PendingOutboundPairingRecord(
      baseUrl: json['baseUrl'] as String,
      state: PairStartState.fromJson(json['state'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'baseUrl': baseUrl,
      'state': state.toJson(),
    };
  }

  /// Rust field `baseUrl` serialized as `baseUrl`.
  final String baseUrl;
  /// Rust field `state` serialized as `state`.
  final PairStartState state;
}

/// Generated Dart model for Rust type `operit_link_access::RemoteDeviceInfo`.
class RemoteDeviceInfo {
  const RemoteDeviceInfo({
    required this.platform,
    required this.model,
  });

  factory RemoteDeviceInfo.fromJson(Map<String, Object?> json) {
    return RemoteDeviceInfo(
      platform: json['platform'] as String,
      model: json['model'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'platform': platform,
      'model': model,
    };
  }

  /// Rust field `platform` serialized as `platform`.
  final String platform;
  /// Rust field `model` serialized as `model`.
  final String model;
}

/// Generated Dart model for Rust type `operit_link_access::RemotePairingCodeRecord`.
class RemotePairingCodeRecord {
  const RemotePairingCodeRecord({
    required this.pairingId,
    required this.pairingServiceVersion,
    required this.clientDeviceId,
    required this.clientDeviceInfo,
    required this.pairingCode,
    required this.createdAt,
  });

  factory RemotePairingCodeRecord.fromJson(Map<String, Object?> json) {
    return RemotePairingCodeRecord(
      pairingId: json['pairingId'] as String,
      pairingServiceVersion: json['pairingServiceVersion'] as int,
      clientDeviceId: json['clientDeviceId'] as String,
      clientDeviceInfo: RemoteDeviceInfo.fromJson(json['clientDeviceInfo'] as Map<String, Object?>),
      pairingCode: json['pairingCode'] as String,
      createdAt: json['createdAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'pairingId': pairingId,
      'pairingServiceVersion': pairingServiceVersion,
      'clientDeviceId': clientDeviceId,
      'clientDeviceInfo': clientDeviceInfo.toJson(),
      'pairingCode': pairingCode,
      'createdAt': createdAt,
    };
  }

  /// Rust field `pairingId` serialized as `pairingId`.
  final String pairingId;
  /// Rust field `pairingServiceVersion` serialized as `pairingServiceVersion`.
  final int pairingServiceVersion;
  /// Rust field `clientDeviceId` serialized as `clientDeviceId`.
  final String clientDeviceId;
  /// Rust field `clientDeviceInfo` serialized as `clientDeviceInfo`.
  final RemoteDeviceInfo clientDeviceInfo;
  /// Rust field `pairingCode` serialized as `pairingCode`.
  final String pairingCode;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
}

/// Generated Dart enum for Rust type `operit_local_models::LocalEngineManifest::LocalArchitecture`.
enum LocalArchitecture {
  aarch64('Aarch64'),
  x8664('X86_64'),
  armv7('Armv7'),
  x86('X86'),
  wasm32('Wasm32'),
  ;

  const LocalArchitecture(this.value);

  final String value;

  factory LocalArchitecture.fromJson(Object? value) {
    return switch (value) {
      'Aarch64' => LocalArchitecture.aarch64,
      'X86_64' => LocalArchitecture.x8664,
      'Armv7' => LocalArchitecture.armv7,
      'X86' => LocalArchitecture.x86,
      'Wasm32' => LocalArchitecture.wasm32,
      _ => throw ArgumentError('Unknown LocalArchitecture: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_local_models::LocalEngineManifest::LocalEngineArchiveFormat`.
enum LocalEngineArchiveFormat {
  tarBz2('TarBz2'),
  ;

  const LocalEngineArchiveFormat(this.value);

  final String value;

  factory LocalEngineArchiveFormat.fromJson(Object? value) {
    return switch (value) {
      'TarBz2' => LocalEngineArchiveFormat.tarBz2,
      _ => throw ArgumentError('Unknown LocalEngineArchiveFormat: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_local_models::LocalEngineManifest::LocalEngineArtifact`.
class LocalEngineArtifact {
  const LocalEngineArtifact({
    required this.target,
    required this.delivery,
    required this.url,
    required this.sha256,
    required this.byteSize,
    required this.archiveFormat,
    required this.archiveRoot,
    required this.sttExecutable,
    required this.ttsExecutable,
    required this.androidLibraryDir,
    required this.ohosLibraryDir,
    required this.iosFrameworkDir,
    required this.webRuntimeDir,
  });

  factory LocalEngineArtifact.fromJson(Map<String, Object?> json) {
    return LocalEngineArtifact(
      target: LocalPlatformTarget.fromJson(json['target'] as Map<String, Object?>),
      delivery: LocalEngineDelivery.fromJson(json['delivery']),
      url: json['url'] as String,
      sha256: json['sha256'] as String,
      byteSize: json['byteSize'] as int,
      archiveFormat: LocalEngineArchiveFormat.fromJson(json['archiveFormat']),
      archiveRoot: json['archiveRoot'] as String,
      sttExecutable: json['sttExecutable'] == null ? null : json['sttExecutable'] as String,
      ttsExecutable: json['ttsExecutable'] == null ? null : json['ttsExecutable'] as String,
      androidLibraryDir: json['androidLibraryDir'] == null ? null : json['androidLibraryDir'] as String,
      ohosLibraryDir: json['ohosLibraryDir'] == null ? null : json['ohosLibraryDir'] as String,
      iosFrameworkDir: json['iosFrameworkDir'] == null ? null : json['iosFrameworkDir'] as String,
      webRuntimeDir: json['webRuntimeDir'] == null ? null : json['webRuntimeDir'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'target': target.toJson(),
      'delivery': delivery.toJson(),
      'url': url,
      'sha256': sha256,
      'byteSize': byteSize,
      'archiveFormat': archiveFormat.toJson(),
      'archiveRoot': archiveRoot,
      'sttExecutable': sttExecutable,
      'ttsExecutable': ttsExecutable,
      'androidLibraryDir': androidLibraryDir,
      'ohosLibraryDir': ohosLibraryDir,
      'iosFrameworkDir': iosFrameworkDir,
      'webRuntimeDir': webRuntimeDir,
    };
  }

  /// Rust field `target` serialized as `target`.
  final LocalPlatformTarget target;
  /// Rust field `delivery` serialized as `delivery`.
  final LocalEngineDelivery delivery;
  /// Rust field `url` serialized as `url`.
  final String url;
  /// Rust field `sha256` serialized as `sha256`.
  final String sha256;
  /// Rust field `byteSize` serialized as `byteSize`.
  final int byteSize;
  /// Rust field `archiveFormat` serialized as `archiveFormat`.
  final LocalEngineArchiveFormat archiveFormat;
  /// Rust field `archiveRoot` serialized as `archiveRoot`.
  final String archiveRoot;
  /// Rust field `sttExecutable` serialized as `sttExecutable`.
  final String? sttExecutable;
  /// Rust field `ttsExecutable` serialized as `ttsExecutable`.
  final String? ttsExecutable;
  /// Rust field `androidLibraryDir` serialized as `androidLibraryDir`.
  final String? androidLibraryDir;
  /// Rust field `ohosLibraryDir` serialized as `ohosLibraryDir`.
  final String? ohosLibraryDir;
  /// Rust field `iosFrameworkDir` serialized as `iosFrameworkDir`.
  final String? iosFrameworkDir;
  /// Rust field `webRuntimeDir` serialized as `webRuntimeDir`.
  final String? webRuntimeDir;
}

/// Generated Dart enum for Rust type `operit_local_models::LocalEngineManifest::LocalEngineDelivery`.
enum LocalEngineDelivery {
  downloadArchive('DownloadArchive'),
  embedded('Embedded'),
  ;

  const LocalEngineDelivery(this.value);

  final String value;

  factory LocalEngineDelivery.fromJson(Object? value) {
    return switch (value) {
      'DownloadArchive' => LocalEngineDelivery.downloadArchive,
      'Embedded' => LocalEngineDelivery.embedded,
      _ => throw ArgumentError('Unknown LocalEngineDelivery: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_local_models::LocalEngineManifest::LocalEngineManifest`.
class LocalEngineManifest {
  const LocalEngineManifest({
    required this.id,
    required this.version,
    required this.displayName,
    required this.description,
    required this.license,
    required this.homepage,
    required this.artifacts,
  });

  factory LocalEngineManifest.fromJson(Map<String, Object?> json) {
    return LocalEngineManifest(
      id: json['id'] as String,
      version: json['version'] as String,
      displayName: json['displayName'] as String,
      description: json['description'] as String,
      license: json['license'] as String,
      homepage: json['homepage'] as String,
      artifacts: (json['artifacts'] as List<Object?>).map((item) => LocalEngineArtifact.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'version': version,
      'displayName': displayName,
      'description': description,
      'license': license,
      'homepage': homepage,
      'artifacts': artifacts.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `version` serialized as `version`.
  final String version;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `license` serialized as `license`.
  final String license;
  /// Rust field `homepage` serialized as `homepage`.
  final String homepage;
  /// Rust field `artifacts` serialized as `artifacts`.
  final List<LocalEngineArtifact> artifacts;
}

/// Generated Dart enum for Rust type `operit_local_models::LocalEngineManifest::LocalPlatform`.
enum LocalPlatform {
  android('Android'),
  ohos('Ohos'),
  ios('Ios'),
  web('Web'),
  windows('Windows'),
  linux('Linux'),
  macos('Macos'),
  ;

  const LocalPlatform(this.value);

  final String value;

  factory LocalPlatform.fromJson(Object? value) {
    return switch (value) {
      'Android' => LocalPlatform.android,
      'Ohos' => LocalPlatform.ohos,
      'Ios' => LocalPlatform.ios,
      'Web' => LocalPlatform.web,
      'Windows' => LocalPlatform.windows,
      'Linux' => LocalPlatform.linux,
      'Macos' => LocalPlatform.macos,
      _ => throw ArgumentError('Unknown LocalPlatform: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_local_models::LocalEngineManifest::LocalPlatformTarget`.
class LocalPlatformTarget {
  const LocalPlatformTarget({
    required this.platform,
    required this.architecture,
  });

  factory LocalPlatformTarget.fromJson(Map<String, Object?> json) {
    return LocalPlatformTarget(
      platform: LocalPlatform.fromJson(json['platform']),
      architecture: LocalArchitecture.fromJson(json['architecture']),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'platform': platform.toJson(),
      'architecture': architecture.toJson(),
    };
  }

  /// Rust field `platform` serialized as `platform`.
  final LocalPlatform platform;
  /// Rust field `architecture` serialized as `architecture`.
  final LocalArchitecture architecture;
}

/// Generated Dart model for Rust type `operit_local_models::LocalInference::LocalModelSelection`.
class LocalModelSelection {
  const LocalModelSelection({
    required this.modelId,
    required this.version,
  });

  factory LocalModelSelection.fromJson(Map<String, Object?> json) {
    return LocalModelSelection(
      modelId: json['modelId'] as String,
      version: json['version'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'modelId': modelId,
      'version': version,
    };
  }

  /// Rust field `modelId` serialized as `modelId`.
  final String modelId;
  /// Rust field `version` serialized as `version`.
  final String version;
}

/// Generated Dart model for Rust type `operit_local_models::LocalInference::LocalSttResponse`.
class LocalSttResponse {
  const LocalSttResponse({
    required this.text,
    required this.segments,
  });

  factory LocalSttResponse.fromJson(Map<String, Object?> json) {
    return LocalSttResponse(
      text: json['text'] as String,
      segments: (json['segments'] as List<Object?>).map((item) => LocalSttSegment.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'text': text,
      'segments': segments.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `text` serialized as `text`.
  final String text;
  /// Rust field `segments` serialized as `segments`.
  final List<LocalSttSegment> segments;
}

/// Generated Dart model for Rust type `operit_local_models::LocalInference::LocalSttSegment`.
class LocalSttSegment {
  const LocalSttSegment({
    required this.text,
    required this.startMs,
    required this.endMs,
  });

  factory LocalSttSegment.fromJson(Map<String, Object?> json) {
    return LocalSttSegment(
      text: json['text'] as String,
      startMs: json['startMs'] as int,
      endMs: json['endMs'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'text': text,
      'startMs': startMs,
      'endMs': endMs,
    };
  }

  /// Rust field `text` serialized as `text`.
  final String text;
  /// Rust field `startMs` serialized as `startMs`.
  final int startMs;
  /// Rust field `endMs` serialized as `endMs`.
  final int endMs;
}

/// Generated Dart model for Rust type `operit_local_models::LocalInference::LocalTtsResponse`.
class LocalTtsResponse {
  const LocalTtsResponse({
    required this.audioBytes,
    required this.outputFormat,
  });

  factory LocalTtsResponse.fromJson(Map<String, Object?> json) {
    return LocalTtsResponse(
      audioBytes: json['audioBytes'] as Uint8List,
      outputFormat: json['outputFormat'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'audioBytes': audioBytes,
      'outputFormat': outputFormat,
    };
  }

  /// Rust field `audioBytes` serialized as `audioBytes`.
  final Uint8List audioBytes;
  /// Rust field `outputFormat` serialized as `outputFormat`.
  final String outputFormat;
}

/// Generated Dart enum for Rust type `operit_local_models::LocalModelManifest::LocalEngineKind`.
enum LocalEngineKind {
  sherpaOnnx('SherpaOnnx'),
  sherpaNcnn('SherpaNcnn'),
  piper('Piper'),
  llamaCpp('LlamaCpp'),
  mnn('Mnn'),
  onnxRuntime('OnnxRuntime'),
  ;

  const LocalEngineKind(this.value);

  final String value;

  factory LocalEngineKind.fromJson(Object? value) {
    return switch (value) {
      'SherpaOnnx' => LocalEngineKind.sherpaOnnx,
      'SherpaNcnn' => LocalEngineKind.sherpaNcnn,
      'Piper' => LocalEngineKind.piper,
      'LlamaCpp' => LocalEngineKind.llamaCpp,
      'Mnn' => LocalEngineKind.mnn,
      'OnnxRuntime' => LocalEngineKind.onnxRuntime,
      _ => throw ArgumentError('Unknown LocalEngineKind: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_local_models::LocalModelManifest::LocalEngineRequirement`.
class LocalEngineRequirement {
  const LocalEngineRequirement({
    required this.engineId,
    required this.version,
  });

  factory LocalEngineRequirement.fromJson(Map<String, Object?> json) {
    return LocalEngineRequirement(
      engineId: json['engineId'] as String,
      version: json['version'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'engineId': engineId,
      'version': version,
    };
  }

  /// Rust field `engineId` serialized as `engineId`.
  final String engineId;
  /// Rust field `version` serialized as `version`.
  final String version;
}

/// Generated Dart model for Rust type `operit_local_models::LocalModelManifest::LocalModelArchive`.
class LocalModelArchive {
  const LocalModelArchive({
    required this.archiveId,
    required this.relativePath,
    required this.sha256,
    required this.byteSize,
    required this.sourceId,
    required this.archiveFormat,
  });

  factory LocalModelArchive.fromJson(Map<String, Object?> json) {
    return LocalModelArchive(
      archiveId: json['archiveId'] as String,
      relativePath: json['relativePath'] as String,
      sha256: json['sha256'] as String,
      byteSize: json['byteSize'] as int,
      sourceId: json['sourceId'] as String,
      archiveFormat: LocalModelArchiveFormat.fromJson(json['archiveFormat']),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'archiveId': archiveId,
      'relativePath': relativePath,
      'sha256': sha256,
      'byteSize': byteSize,
      'sourceId': sourceId,
      'archiveFormat': archiveFormat.toJson(),
    };
  }

  /// Rust field `archiveId` serialized as `archiveId`.
  final String archiveId;
  /// Rust field `relativePath` serialized as `relativePath`.
  final String relativePath;
  /// Rust field `sha256` serialized as `sha256`.
  final String sha256;
  /// Rust field `byteSize` serialized as `byteSize`.
  final int byteSize;
  /// Rust field `sourceId` serialized as `sourceId`.
  final String sourceId;
  /// Rust field `archiveFormat` serialized as `archiveFormat`.
  final LocalModelArchiveFormat archiveFormat;
}

/// Generated Dart enum for Rust type `operit_local_models::LocalModelManifest::LocalModelArchiveFormat`.
enum LocalModelArchiveFormat {
  tarBz2('TarBz2'),
  ;

  const LocalModelArchiveFormat(this.value);

  final String value;

  factory LocalModelArchiveFormat.fromJson(Object? value) {
    return switch (value) {
      'TarBz2' => LocalModelArchiveFormat.tarBz2,
      _ => throw ArgumentError('Unknown LocalModelArchiveFormat: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart tagged enum model for Rust type `operit_local_models::LocalModelManifest::LocalModelDriver`.
class LocalModelDriver {
  const LocalModelDriver._({
    this.encoder = '',
    this.decoder = '',
    this.joiner = '',
    this.tokens = '',
    this.modelType = '',
    this.model = '',
    this.lexicon = '',
    this.ruleFsts = const [],
    this.ruleFars = const [],
    this.speakerCount = 0,
    this.acousticModel = '',
    this.vocoder = '',
    this.voices = '',
    this.dataDir = '',
    this.recognizerScript = '',
    this.runtimeScript = '',
    this.runtimeWasm = '',
    this.runtimeData = '',
    this.ttsScript = '',
    this.encoderParam = '',
    this.encoderBin = '',
    this.decoderParam = '',
    this.decoderBin = '',
    this.joinerParam = '',
    this.joinerBin = '',
    required this.tag,
  });

  factory LocalModelDriver.sherpaOnnxStreamingTransducer({required String encoder, required String decoder, required String joiner, required String tokens, required String modelType, }) => LocalModelDriver._(tag: 'SherpaOnnxStreamingTransducer', encoder: encoder, decoder: decoder, joiner: joiner, tokens: tokens, modelType: modelType);
  factory LocalModelDriver.sherpaOnnxVits({required String model, required String lexicon, required String tokens, required List<String> ruleFsts, required List<String> ruleFars, required int speakerCount, }) => LocalModelDriver._(tag: 'SherpaOnnxVits', model: model, lexicon: lexicon, tokens: tokens, ruleFsts: ruleFsts, ruleFars: ruleFars, speakerCount: speakerCount);
  factory LocalModelDriver.sherpaOnnxMatcha({required String acousticModel, required String vocoder, required String lexicon, required String tokens, required List<String> ruleFsts, required List<String> ruleFars, required int speakerCount, }) => LocalModelDriver._(tag: 'SherpaOnnxMatcha', acousticModel: acousticModel, vocoder: vocoder, lexicon: lexicon, tokens: tokens, ruleFsts: ruleFsts, ruleFars: ruleFars, speakerCount: speakerCount);
  factory LocalModelDriver.sherpaOnnxKitten({required String model, required String voices, required String tokens, required String dataDir, required int speakerCount, }) => LocalModelDriver._(tag: 'SherpaOnnxKitten', model: model, voices: voices, tokens: tokens, dataDir: dataDir, speakerCount: speakerCount);
  factory LocalModelDriver.sherpaOnnxWebAsrBundle({required String recognizerScript, required String runtimeScript, required String runtimeWasm, required String runtimeData, }) => LocalModelDriver._(tag: 'SherpaOnnxWebAsrBundle', recognizerScript: recognizerScript, runtimeScript: runtimeScript, runtimeWasm: runtimeWasm, runtimeData: runtimeData);
  factory LocalModelDriver.sherpaOnnxWebTtsBundle({required String ttsScript, required String runtimeScript, required String runtimeWasm, required String runtimeData, required int speakerCount, }) => LocalModelDriver._(tag: 'SherpaOnnxWebTtsBundle', ttsScript: ttsScript, runtimeScript: runtimeScript, runtimeWasm: runtimeWasm, runtimeData: runtimeData, speakerCount: speakerCount);
  factory LocalModelDriver.sherpaNcnnStreamingTransducer({required String encoderParam, required String encoderBin, required String decoderParam, required String decoderBin, required String joinerParam, required String joinerBin, required String tokens, }) => LocalModelDriver._(tag: 'SherpaNcnnStreamingTransducer', encoderParam: encoderParam, encoderBin: encoderBin, decoderParam: decoderParam, decoderBin: decoderBin, joinerParam: joinerParam, joinerBin: joinerBin, tokens: tokens);
  final String tag;
  final String encoder;
  final String decoder;
  final String joiner;
  final String tokens;
  final String modelType;
  final String model;
  final String lexicon;
  final List<String> ruleFsts;
  final List<String> ruleFars;
  final int speakerCount;
  final String acousticModel;
  final String vocoder;
  final String voices;
  final String dataDir;
  final String recognizerScript;
  final String runtimeScript;
  final String runtimeWasm;
  final String runtimeData;
  final String ttsScript;
  final String encoderParam;
  final String encoderBin;
  final String decoderParam;
  final String decoderBin;
  final String joinerParam;
  final String joinerBin;

  factory LocalModelDriver.fromJson(Object? json) {
    final map = json as Map<String, Object?>;
    final tag = map.keys.first;
    final data = map[tag] as Map<String, Object?>? ?? <String, Object?>{};
    return switch (tag) {
      'SherpaOnnxStreamingTransducer' => LocalModelDriver.sherpaOnnxStreamingTransducer(encoder: data['encoder'] as String, decoder: data['decoder'] as String, joiner: data['joiner'] as String, tokens: data['tokens'] as String, modelType: data['modelType'] as String, ),
      'SherpaOnnxVits' => LocalModelDriver.sherpaOnnxVits(model: data['model'] as String, lexicon: data['lexicon'] as String, tokens: data['tokens'] as String, ruleFsts: (data['ruleFsts'] as List<Object?>).map((item) => item as String).toList(growable: false), ruleFars: (data['ruleFars'] as List<Object?>).map((item) => item as String).toList(growable: false), speakerCount: data['speakerCount'] as int, ),
      'SherpaOnnxMatcha' => LocalModelDriver.sherpaOnnxMatcha(acousticModel: data['acousticModel'] as String, vocoder: data['vocoder'] as String, lexicon: data['lexicon'] as String, tokens: data['tokens'] as String, ruleFsts: (data['ruleFsts'] as List<Object?>).map((item) => item as String).toList(growable: false), ruleFars: (data['ruleFars'] as List<Object?>).map((item) => item as String).toList(growable: false), speakerCount: data['speakerCount'] as int, ),
      'SherpaOnnxKitten' => LocalModelDriver.sherpaOnnxKitten(model: data['model'] as String, voices: data['voices'] as String, tokens: data['tokens'] as String, dataDir: data['dataDir'] as String, speakerCount: data['speakerCount'] as int, ),
      'SherpaOnnxWebAsrBundle' => LocalModelDriver.sherpaOnnxWebAsrBundle(recognizerScript: data['recognizerScript'] as String, runtimeScript: data['runtimeScript'] as String, runtimeWasm: data['runtimeWasm'] as String, runtimeData: data['runtimeData'] as String, ),
      'SherpaOnnxWebTtsBundle' => LocalModelDriver.sherpaOnnxWebTtsBundle(ttsScript: data['ttsScript'] as String, runtimeScript: data['runtimeScript'] as String, runtimeWasm: data['runtimeWasm'] as String, runtimeData: data['runtimeData'] as String, speakerCount: data['speakerCount'] as int, ),
      'SherpaNcnnStreamingTransducer' => LocalModelDriver.sherpaNcnnStreamingTransducer(encoderParam: data['encoderParam'] as String, encoderBin: data['encoderBin'] as String, decoderParam: data['decoderParam'] as String, decoderBin: data['decoderBin'] as String, joinerParam: data['joinerParam'] as String, joinerBin: data['joinerBin'] as String, tokens: data['tokens'] as String, ),
      _ => throw ArgumentError('Unknown LocalModelDriver tag: $tag'),
    };
  }

  Map<String, Object?> toJson() {
    final data = <String, Object?>{
      if (tag == 'SherpaOnnxStreamingTransducer') ...<String, Object?>{
        'encoder': encoder,
        'decoder': decoder,
        'joiner': joiner,
        'tokens': tokens,
        'modelType': modelType,
      },
      if (tag == 'SherpaOnnxVits') ...<String, Object?>{
        'model': model,
        'lexicon': lexicon,
        'tokens': tokens,
        'ruleFsts': ruleFsts.map((item) => item).toList(growable: false),
        'ruleFars': ruleFars.map((item) => item).toList(growable: false),
        'speakerCount': speakerCount,
      },
      if (tag == 'SherpaOnnxMatcha') ...<String, Object?>{
        'acousticModel': acousticModel,
        'vocoder': vocoder,
        'lexicon': lexicon,
        'tokens': tokens,
        'ruleFsts': ruleFsts.map((item) => item).toList(growable: false),
        'ruleFars': ruleFars.map((item) => item).toList(growable: false),
        'speakerCount': speakerCount,
      },
      if (tag == 'SherpaOnnxKitten') ...<String, Object?>{
        'model': model,
        'voices': voices,
        'tokens': tokens,
        'dataDir': dataDir,
        'speakerCount': speakerCount,
      },
      if (tag == 'SherpaOnnxWebAsrBundle') ...<String, Object?>{
        'recognizerScript': recognizerScript,
        'runtimeScript': runtimeScript,
        'runtimeWasm': runtimeWasm,
        'runtimeData': runtimeData,
      },
      if (tag == 'SherpaOnnxWebTtsBundle') ...<String, Object?>{
        'ttsScript': ttsScript,
        'runtimeScript': runtimeScript,
        'runtimeWasm': runtimeWasm,
        'runtimeData': runtimeData,
        'speakerCount': speakerCount,
      },
      if (tag == 'SherpaNcnnStreamingTransducer') ...<String, Object?>{
        'encoderParam': encoderParam,
        'encoderBin': encoderBin,
        'decoderParam': decoderParam,
        'decoderBin': decoderBin,
        'joinerParam': joinerParam,
        'joinerBin': joinerBin,
        'tokens': tokens,
      },
    };
    return <String, Object?>{tag: data};
  }
}

/// Generated Dart model for Rust type `operit_local_models::LocalModelManifest::LocalModelFile`.
class LocalModelFile {
  const LocalModelFile({
    required this.relativePath,
    required this.sha256,
    required this.byteSize,
    required this.sourceId,
  });

  factory LocalModelFile.fromJson(Map<String, Object?> json) {
    return LocalModelFile(
      relativePath: json['relativePath'] as String,
      sha256: json['sha256'] as String,
      byteSize: json['byteSize'] as int,
      sourceId: json['sourceId'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'relativePath': relativePath,
      'sha256': sha256,
      'byteSize': byteSize,
      'sourceId': sourceId,
    };
  }

  /// Rust field `relativePath` serialized as `relativePath`.
  final String relativePath;
  /// Rust field `sha256` serialized as `sha256`.
  final String sha256;
  /// Rust field `byteSize` serialized as `byteSize`.
  final int byteSize;
  /// Rust field `sourceId` serialized as `sourceId`.
  final String sourceId;
}

/// Generated Dart tagged enum model for Rust type `operit_local_models::LocalModelManifest::LocalModelInstallSource`.
class LocalModelInstallSource {
  const LocalModelInstallSource._({
    this.archives = const [],
    required this.tag,
  });

  factory LocalModelInstallSource.files() => LocalModelInstallSource._(tag: 'Files');
  factory LocalModelInstallSource.archives({required List<LocalModelArchive> archives, }) => LocalModelInstallSource._(tag: 'Archives', archives: archives);
  final String tag;
  final List<LocalModelArchive> archives;

  factory LocalModelInstallSource.fromJson(Object? json) {
    switch (json) {
      case 'Files':
        return LocalModelInstallSource.files();
      case Map<String, Object?> map:
        final tag = map.keys.single;
        final data = map[tag] as Map<String, Object?>;
        return switch (tag) {
          'Archives' => LocalModelInstallSource.archives(archives: (data['archives'] as List<Object?>).map((item) => LocalModelArchive.fromJson(item as Map<String, Object?>)).toList(growable: false), ),
          _ => throw ArgumentError('Unknown LocalModelInstallSource tag: $tag'),
        };
      default:
        throw ArgumentError('Unknown LocalModelInstallSource representation: $json');
    }
  }

  Object toJson() {
    return switch (tag) {
      'Files' => 'Files',
      'Archives' => <String, Object?>{'Archives': <String, Object?>{
        'archives': archives.map((item) => item.toJson()).toList(growable: false),
      }},
      _ => throw StateError('Unknown LocalModelInstallSource tag: $tag'),
    };
  }
}

/// Generated Dart enum for Rust type `operit_local_models::LocalModelManifest::LocalModelKind`.
enum LocalModelKind {
  speechToText('SpeechToText'),
  textToSpeech('TextToSpeech'),
  chat('Chat'),
  embedding('Embedding'),
  ;

  const LocalModelKind(this.value);

  final String value;

  factory LocalModelKind.fromJson(Object? value) {
    return switch (value) {
      'SpeechToText' => LocalModelKind.speechToText,
      'TextToSpeech' => LocalModelKind.textToSpeech,
      'Chat' => LocalModelKind.chat,
      'Embedding' => LocalModelKind.embedding,
      _ => throw ArgumentError('Unknown LocalModelKind: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_local_models::LocalModelManifest::LocalModelManifest`.
class LocalModelManifest {
  const LocalModelManifest({
    required this.id,
    required this.version,
    required this.displayName,
    required this.description,
    required this.kind,
    required this.engine,
    required this.license,
    required this.homepage,
    required this.languages,
    required this.tags,
    required this.engineRequirement,
    required this.driver,
    required this.sources,
    required this.installSource,
    required this.files,
  });

  factory LocalModelManifest.fromJson(Map<String, Object?> json) {
    return LocalModelManifest(
      id: json['id'] as String,
      version: json['version'] as String,
      displayName: json['displayName'] as String,
      description: json['description'] as String,
      kind: LocalModelKind.fromJson(json['kind']),
      engine: LocalEngineKind.fromJson(json['engine']),
      license: json['license'] as String,
      homepage: json['homepage'] as String,
      languages: (json['languages'] as List<Object?>).map((item) => item as String).toList(growable: false),
      tags: (json['tags'] as List<Object?>).map((item) => item as String).toList(growable: false),
      engineRequirement: json['engineRequirement'] == null ? null : LocalEngineRequirement.fromJson(json['engineRequirement'] as Map<String, Object?>),
      driver: json['driver'] == null ? null : LocalModelDriver.fromJson(json['driver']),
      sources: (json['sources'] as List<Object?>).map((item) => LocalModelSource.fromJson(item as Map<String, Object?>)).toList(growable: false),
      installSource: LocalModelInstallSource.fromJson(json['installSource']),
      files: (json['files'] as List<Object?>).map((item) => LocalModelFile.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'version': version,
      'displayName': displayName,
      'description': description,
      'kind': kind.toJson(),
      'engine': engine.toJson(),
      'license': license,
      'homepage': homepage,
      'languages': languages.map((item) => item).toList(growable: false),
      'tags': tags.map((item) => item).toList(growable: false),
      'engineRequirement': engineRequirement?.toJson(),
      'driver': driver?.toJson(),
      'sources': sources.map((item) => item.toJson()).toList(growable: false),
      'installSource': installSource.toJson(),
      'files': files.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `version` serialized as `version`.
  final String version;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `kind` serialized as `kind`.
  final LocalModelKind kind;
  /// Rust field `engine` serialized as `engine`.
  final LocalEngineKind engine;
  /// Rust field `license` serialized as `license`.
  final String license;
  /// Rust field `homepage` serialized as `homepage`.
  final String homepage;
  /// Rust field `languages` serialized as `languages`.
  final List<String> languages;
  /// Rust field `tags` serialized as `tags`.
  final List<String> tags;
  /// Rust field `engineRequirement` serialized as `engineRequirement`.
  final LocalEngineRequirement? engineRequirement;
  /// Rust field `driver` serialized as `driver`.
  final LocalModelDriver? driver;
  /// Rust field `sources` serialized as `sources`.
  final List<LocalModelSource> sources;
  /// Rust field `installSource` serialized as `installSource`.
  final LocalModelInstallSource installSource;
  /// Rust field `files` serialized as `files`.
  final List<LocalModelFile> files;
}

/// Generated Dart model for Rust type `operit_local_models::LocalModelManifest::LocalModelSource`.
class LocalModelSource {
  const LocalModelSource({
    required this.id,
    required this.kind,
    required this.repository,
    required this.revision,
    required this.baseUrl,
  });

  factory LocalModelSource.fromJson(Map<String, Object?> json) {
    return LocalModelSource(
      id: json['id'] as String,
      kind: LocalModelSourceKind.fromJson(json['kind']),
      repository: json['repository'] as String,
      revision: json['revision'] as String,
      baseUrl: json['baseUrl'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'kind': kind.toJson(),
      'repository': repository,
      'revision': revision,
      'baseUrl': baseUrl,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `kind` serialized as `kind`.
  final LocalModelSourceKind kind;
  /// Rust field `repository` serialized as `repository`.
  final String repository;
  /// Rust field `revision` serialized as `revision`.
  final String revision;
  /// Rust field `baseUrl` serialized as `baseUrl`.
  final String baseUrl;
}

/// Generated Dart enum for Rust type `operit_local_models::LocalModelManifest::LocalModelSourceKind`.
enum LocalModelSourceKind {
  huggingFace('HuggingFace'),
  modelScope('ModelScope'),
  directHttp('DirectHttp'),
  ;

  const LocalModelSourceKind(this.value);

  final String value;

  factory LocalModelSourceKind.fromJson(Object? value) {
    return switch (value) {
      'HuggingFace' => LocalModelSourceKind.huggingFace,
      'ModelScope' => LocalModelSourceKind.modelScope,
      'DirectHttp' => LocalModelSourceKind.directHttp,
      _ => throw ArgumentError('Unknown LocalModelSourceKind: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_local_models::LocalModelRegistry::InstalledLocalEngine`.
class InstalledLocalEngine {
  const InstalledLocalEngine({
    required this.manifest,
    required this.artifact,
    required this.storagePath,
    required this.installedAtMs,
    required this.verifiedAtMs,
  });

  factory InstalledLocalEngine.fromJson(Map<String, Object?> json) {
    return InstalledLocalEngine(
      manifest: LocalEngineManifest.fromJson(json['manifest'] as Map<String, Object?>),
      artifact: LocalEngineArtifact.fromJson(json['artifact'] as Map<String, Object?>),
      storagePath: json['storagePath'] as String,
      installedAtMs: json['installedAtMs'] as int,
      verifiedAtMs: json['verifiedAtMs'] == null ? null : json['verifiedAtMs'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'manifest': manifest.toJson(),
      'artifact': artifact.toJson(),
      'storagePath': storagePath,
      'installedAtMs': installedAtMs,
      'verifiedAtMs': verifiedAtMs,
    };
  }

  /// Rust field `manifest` serialized as `manifest`.
  final LocalEngineManifest manifest;
  /// Rust field `artifact` serialized as `artifact`.
  final LocalEngineArtifact artifact;
  /// Rust field `storagePath` serialized as `storagePath`.
  final String storagePath;
  /// Rust field `installedAtMs` serialized as `installedAtMs`.
  final int installedAtMs;
  /// Rust field `verifiedAtMs` serialized as `verifiedAtMs`.
  final int? verifiedAtMs;
}

/// Generated Dart model for Rust type `operit_local_models::LocalModelRegistry::InstalledLocalModel`.
class InstalledLocalModel {
  const InstalledLocalModel({
    required this.manifest,
    required this.storagePath,
    required this.installedAtMs,
    required this.verifiedAtMs,
  });

  factory InstalledLocalModel.fromJson(Map<String, Object?> json) {
    return InstalledLocalModel(
      manifest: LocalModelManifest.fromJson(json['manifest'] as Map<String, Object?>),
      storagePath: json['storagePath'] as String,
      installedAtMs: json['installedAtMs'] as int,
      verifiedAtMs: json['verifiedAtMs'] == null ? null : json['verifiedAtMs'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'manifest': manifest.toJson(),
      'storagePath': storagePath,
      'installedAtMs': installedAtMs,
      'verifiedAtMs': verifiedAtMs,
    };
  }

  /// Rust field `manifest` serialized as `manifest`.
  final LocalModelManifest manifest;
  /// Rust field `storagePath` serialized as `storagePath`.
  final String storagePath;
  /// Rust field `installedAtMs` serialized as `installedAtMs`.
  final int installedAtMs;
  /// Rust field `verifiedAtMs` serialized as `verifiedAtMs`.
  final int? verifiedAtMs;
}

/// Generated Dart model for Rust type `operit_local_models::LocalModelRegistry::LocalModelRegistrySnapshot`.
class LocalModelRegistrySnapshot {
  const LocalModelRegistrySnapshot({
    required this.installedModels,
    required this.installedEngines,
  });

  factory LocalModelRegistrySnapshot.fromJson(Map<String, Object?> json) {
    return LocalModelRegistrySnapshot(
      installedModels: (json['installedModels'] as List<Object?>).map((item) => InstalledLocalModel.fromJson(item as Map<String, Object?>)).toList(growable: false),
      installedEngines: (json['installedEngines'] as List<Object?>).map((item) => InstalledLocalEngine.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'installedModels': installedModels.map((item) => item.toJson()).toList(growable: false),
      'installedEngines': installedEngines.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `installedModels` serialized as `installedModels`.
  final List<InstalledLocalModel> installedModels;
  /// Rust field `installedEngines` serialized as `installedEngines`.
  final List<InstalledLocalEngine> installedEngines;
}

/// Generated Dart tagged enum model for Rust type `operit_model::ActivePrompt::ActivePrompt`.
class ActivePrompt {
  const ActivePrompt._({
    this.id = '',
    required this.tag,
  });

  factory ActivePrompt.characterCard({required String id, }) => ActivePrompt._(tag: 'CharacterCard', id: id);
  factory ActivePrompt.characterGroup({required String id, }) => ActivePrompt._(tag: 'CharacterGroup', id: id);
  final String tag;
  final String id;

  factory ActivePrompt.fromJson(Object? json) {
    final map = json as Map<String, Object?>;
    final tag = map.keys.first;
    final data = map[tag] as Map<String, Object?>? ?? <String, Object?>{};
    return switch (tag) {
      'CharacterCard' => ActivePrompt.characterCard(id: data['id'] as String, ),
      'CharacterGroup' => ActivePrompt.characterGroup(id: data['id'] as String, ),
      _ => throw ArgumentError('Unknown ActivePrompt tag: $tag'),
    };
  }

  Map<String, Object?> toJson() {
    final data = <String, Object?>{
      if (tag == 'CharacterCard') ...<String, Object?>{
        'id': id,
      },
      if (tag == 'CharacterGroup') ...<String, Object?>{
        'id': id,
      },
    };
    return <String, Object?>{tag: data};
  }
}

/// Generated Dart model for Rust type `operit_model::AttachmentInfo::AttachmentInfo`.
class AttachmentInfo {
  const AttachmentInfo({
    required this.filePath,
    required this.fileName,
    required this.mimeType,
    required this.fileSize,
    required this.content,
  });

  factory AttachmentInfo.fromJson(Map<String, Object?> json) {
    return AttachmentInfo(
      filePath: json['filePath'] as String,
      fileName: json['fileName'] as String,
      mimeType: json['mimeType'] as String,
      fileSize: json['fileSize'] as int,
      content: json['content'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'filePath': filePath,
      'fileName': fileName,
      'mimeType': mimeType,
      'fileSize': fileSize,
      'content': content,
    };
  }

  /// Rust field `filePath` serialized as `filePath`.
  final String filePath;
  /// Rust field `fileName` serialized as `fileName`.
  final String fileName;
  /// Rust field `mimeType` serialized as `mimeType`.
  final String mimeType;
  /// Rust field `fileSize` serialized as `fileSize`.
  final int fileSize;
  /// Rust field `content` serialized as `content`.
  final String content;
}

/// Generated Dart model for Rust type `operit_model::CharacterCard::CharacterCard`.
class CharacterCard {
  const CharacterCard({
    required this.id,
    required this.name,
    required this.description,
    required this.characterSetting,
    required this.openingStatement,
    required this.otherContentChat,
    required this.otherContentVoice,
    required this.avatarUri,
    required this.attachedTagIds,
    required this.advancedCustomPrompt,
    required this.marks,
    required this.chatModelBindingMode,
    required this.chatModelId,
    required this.ttsConfigId,
    required this.memoryBindingMode,
    required this.sharedMemoryId,
    required this.sharedMemoryMounts,
    required this.toolAccessConfig,
    required this.isDefault,
    required this.createdAt,
    required this.updatedAt,
  });

  factory CharacterCard.fromJson(Map<String, Object?> json) {
    return CharacterCard(
      id: json['id'] as String,
      name: json['name'] as String,
      description: json['description'] as String,
      characterSetting: json['characterSetting'] as String,
      openingStatement: json['openingStatement'] as String,
      otherContentChat: json['otherContentChat'] as String,
      otherContentVoice: json['otherContentVoice'] as String,
      avatarUri: json['avatarUri'] == null ? null : json['avatarUri'] as String,
      attachedTagIds: (json['attachedTagIds'] as List<Object?>).map((item) => item as String).toList(growable: false),
      advancedCustomPrompt: json['advancedCustomPrompt'] as String,
      marks: json['marks'] as String,
      chatModelBindingMode: json['chatModelBindingMode'] as String,
      chatModelId: json['chatModelId'] == null ? null : json['chatModelId'] as String,
      ttsConfigId: json['ttsConfigId'] == null ? null : json['ttsConfigId'] as String,
      memoryBindingMode: json['memoryBindingMode'] as String,
      sharedMemoryId: json['sharedMemoryId'] == null ? null : json['sharedMemoryId'] as String,
      sharedMemoryMounts: (json['sharedMemoryMounts'] as List<Object?>).map((item) => CharacterSharedMemoryMount.fromJson(item as Map<String, Object?>)).toList(growable: false),
      toolAccessConfig: CharacterCardToolAccessConfig.fromJson(json['toolAccessConfig'] as Map<String, Object?>),
      isDefault: json['isDefault'] as bool,
      createdAt: json['createdAt'] as int,
      updatedAt: json['updatedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
      'description': description,
      'characterSetting': characterSetting,
      'openingStatement': openingStatement,
      'otherContentChat': otherContentChat,
      'otherContentVoice': otherContentVoice,
      'avatarUri': avatarUri,
      'attachedTagIds': attachedTagIds.map((item) => item).toList(growable: false),
      'advancedCustomPrompt': advancedCustomPrompt,
      'marks': marks,
      'chatModelBindingMode': chatModelBindingMode,
      'chatModelId': chatModelId,
      'ttsConfigId': ttsConfigId,
      'memoryBindingMode': memoryBindingMode,
      'sharedMemoryId': sharedMemoryId,
      'sharedMemoryMounts': sharedMemoryMounts.map((item) => item.toJson()).toList(growable: false),
      'toolAccessConfig': toolAccessConfig.toJson(),
      'isDefault': isDefault,
      'createdAt': createdAt,
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `characterSetting` serialized as `characterSetting`.
  final String characterSetting;
  /// Rust field `openingStatement` serialized as `openingStatement`.
  final String openingStatement;
  /// Rust field `otherContentChat` serialized as `otherContentChat`.
  final String otherContentChat;
  /// Rust field `otherContentVoice` serialized as `otherContentVoice`.
  final String otherContentVoice;
  /// Rust field `avatarUri` serialized as `avatarUri`.
  final String? avatarUri;
  /// Rust field `attachedTagIds` serialized as `attachedTagIds`.
  final List<String> attachedTagIds;
  /// Rust field `advancedCustomPrompt` serialized as `advancedCustomPrompt`.
  final String advancedCustomPrompt;
  /// Rust field `marks` serialized as `marks`.
  final String marks;
  /// Rust field `chatModelBindingMode` serialized as `chatModelBindingMode`.
  final String chatModelBindingMode;
  /// Rust field `chatModelId` serialized as `chatModelId`.
  final String? chatModelId;
  /// Rust field `ttsConfigId` serialized as `ttsConfigId`.
  final String? ttsConfigId;
  /// Rust field `memoryBindingMode` serialized as `memoryBindingMode`.
  final String memoryBindingMode;
  /// Rust field `sharedMemoryId` serialized as `sharedMemoryId`.
  final String? sharedMemoryId;
  /// Rust field `sharedMemoryMounts` serialized as `sharedMemoryMounts`.
  final List<CharacterSharedMemoryMount> sharedMemoryMounts;
  /// Rust field `toolAccessConfig` serialized as `toolAccessConfig`.
  final CharacterCardToolAccessConfig toolAccessConfig;
  /// Rust field `isDefault` serialized as `isDefault`.
  final bool isDefault;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
}

/// Generated Dart model for Rust type `operit_model::CharacterCard::CharacterCardToolAccessConfig`.
class CharacterCardToolAccessConfig {
  const CharacterCardToolAccessConfig({
    required this.enabled,
    required this.allowedBuiltinTools,
    required this.allowedPackages,
    required this.allowedSkills,
    required this.allowedMcpServers,
  });

  factory CharacterCardToolAccessConfig.fromJson(Map<String, Object?> json) {
    return CharacterCardToolAccessConfig(
      enabled: json['enabled'] as bool,
      allowedBuiltinTools: (json['allowedBuiltinTools'] as List<Object?>).map((item) => item as String).toList(growable: false),
      allowedPackages: (json['allowedPackages'] as List<Object?>).map((item) => item as String).toList(growable: false),
      allowedSkills: (json['allowedSkills'] as List<Object?>).map((item) => item as String).toList(growable: false),
      allowedMcpServers: (json['allowedMcpServers'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'enabled': enabled,
      'allowedBuiltinTools': allowedBuiltinTools.map((item) => item).toList(growable: false),
      'allowedPackages': allowedPackages.map((item) => item).toList(growable: false),
      'allowedSkills': allowedSkills.map((item) => item).toList(growable: false),
      'allowedMcpServers': allowedMcpServers.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `enabled` serialized as `enabled`.
  final bool enabled;
  /// Rust field `allowedBuiltinTools` serialized as `allowedBuiltinTools`.
  final List<String> allowedBuiltinTools;
  /// Rust field `allowedPackages` serialized as `allowedPackages`.
  final List<String> allowedPackages;
  /// Rust field `allowedSkills` serialized as `allowedSkills`.
  final List<String> allowedSkills;
  /// Rust field `allowedMcpServers` serialized as `allowedMcpServers`.
  final List<String> allowedMcpServers;
}

/// Generated Dart model for Rust type `operit_model::CharacterCard::CharacterSharedMemoryMount`.
class CharacterSharedMemoryMount {
  const CharacterSharedMemoryMount({
    required this.sharedMemoryId,
    required this.readable,
    required this.writable,
  });

  factory CharacterSharedMemoryMount.fromJson(Map<String, Object?> json) {
    return CharacterSharedMemoryMount(
      sharedMemoryId: json['sharedMemoryId'] as String,
      readable: json['readable'] as bool,
      writable: json['writable'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sharedMemoryId': sharedMemoryId,
      'readable': readable,
      'writable': writable,
    };
  }

  /// Rust field `sharedMemoryId` serialized as `sharedMemoryId`.
  final String sharedMemoryId;
  /// Rust field `readable` serialized as `readable`.
  final bool readable;
  /// Rust field `writable` serialized as `writable`.
  final bool writable;
}

/// Generated Dart model for Rust type `operit_model::CharacterGroupCard::CharacterGroupCard`.
class CharacterGroupCard {
  const CharacterGroupCard({
    required this.id,
    required this.name,
    required this.description,
    required this.members,
    required this.createdAt,
    required this.updatedAt,
  });

  factory CharacterGroupCard.fromJson(Map<String, Object?> json) {
    return CharacterGroupCard(
      id: json['id'] as String,
      name: json['name'] as String,
      description: json['description'] as String,
      members: (json['members'] as List<Object?>).map((item) => GroupMemberConfig.fromJson(item as Map<String, Object?>)).toList(growable: false),
      createdAt: json['createdAt'] as int,
      updatedAt: json['updatedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
      'description': description,
      'members': members.map((item) => item.toJson()).toList(growable: false),
      'createdAt': createdAt,
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `members` serialized as `members`.
  final List<GroupMemberConfig> members;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
}

/// Generated Dart model for Rust type `operit_model::CharacterGroupCard::GroupMemberConfig`.
class GroupMemberConfig {
  const GroupMemberConfig({
    required this.characterCardId,
    required this.orderIndex,
  });

  factory GroupMemberConfig.fromJson(Map<String, Object?> json) {
    return GroupMemberConfig(
      characterCardId: json['characterCardId'] as String,
      orderIndex: json['orderIndex'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'characterCardId': characterCardId,
      'orderIndex': orderIndex,
    };
  }

  /// Rust field `characterCardId` serialized as `characterCardId`.
  final String characterCardId;
  /// Rust field `orderIndex` serialized as `orderIndex`.
  final int orderIndex;
}

/// Generated Dart model for Rust type `operit_model::ChatHistory::ChatHistory`.
class ChatHistory {
  const ChatHistory({
    required this.id,
    required this.title,
    required this.messages,
    required this.createdAt,
    required this.updatedAt,
    required this.inputTokens,
    required this.outputTokens,
    required this.currentWindowSize,
    required this.group,
    required this.displayOrder,
    required this.workspace,
    required this.parentChatId,
    required this.characterCardName,
    required this.characterGroupId,
    required this.locked,
    required this.pinned,
  });

  factory ChatHistory.fromJson(Map<String, Object?> json) {
    return ChatHistory(
      id: json['id'] as String,
      title: json['title'] as String,
      messages: (json['messages'] as List<Object?>).map((item) => item).toList(growable: false),
      createdAt: json['createdAt'] as String,
      updatedAt: json['updatedAt'] as String,
      inputTokens: json['inputTokens'] as int,
      outputTokens: json['outputTokens'] as int,
      currentWindowSize: json['currentWindowSize'] as int,
      group: json['group'] == null ? null : json['group'] as String,
      displayOrder: json['displayOrder'] as int,
      workspace: json['workspace'] == null ? null : json['workspace'] as String,
      parentChatId: json['parentChatId'] == null ? null : json['parentChatId'] as String,
      characterCardName: json['characterCardName'] == null ? null : json['characterCardName'] as String,
      characterGroupId: json['characterGroupId'] == null ? null : json['characterGroupId'] as String,
      locked: json['locked'] as bool,
      pinned: json['pinned'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'title': title,
      'messages': messages.map((item) => item).toList(growable: false),
      'createdAt': createdAt,
      'updatedAt': updatedAt,
      'inputTokens': inputTokens,
      'outputTokens': outputTokens,
      'currentWindowSize': currentWindowSize,
      'group': group,
      'displayOrder': displayOrder,
      'workspace': workspace,
      'parentChatId': parentChatId,
      'characterCardName': characterCardName,
      'characterGroupId': characterGroupId,
      'locked': locked,
      'pinned': pinned,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `messages` serialized as `messages`.
  final List<Object?> messages;
  /// Rust field `createdAt` serialized as `createdAt`.
  final String createdAt;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final String updatedAt;
  /// Rust field `inputTokens` serialized as `inputTokens`.
  final int inputTokens;
  /// Rust field `outputTokens` serialized as `outputTokens`.
  final int outputTokens;
  /// Rust field `currentWindowSize` serialized as `currentWindowSize`.
  final int currentWindowSize;
  /// Rust field `group` serialized as `group`.
  final String? group;
  /// Rust field `displayOrder` serialized as `displayOrder`.
  final int displayOrder;
  /// Rust field `workspace` serialized as `workspace`.
  final String? workspace;
  /// Rust field `parentChatId` serialized as `parentChatId`.
  final String? parentChatId;
  /// Rust field `characterCardName` serialized as `characterCardName`.
  final String? characterCardName;
  /// Rust field `characterGroupId` serialized as `characterGroupId`.
  final String? characterGroupId;
  /// Rust field `locked` serialized as `locked`.
  final bool locked;
  /// Rust field `pinned` serialized as `pinned`.
  final bool pinned;
}

/// Generated Dart model for Rust type `operit_model::ChatHistoryListItem::ChatHistoryListItem`.
class ChatHistoryListItem {
  const ChatHistoryListItem({
    required this.id,
    required this.title,
    required this.updatedAt,
    required this.group,
    required this.displayOrder,
    required this.characterCardName,
    required this.characterGroupId,
    required this.locked,
    required this.pinned,
  });

  factory ChatHistoryListItem.fromJson(Map<String, Object?> json) {
    return ChatHistoryListItem(
      id: json['id'] as String,
      title: json['title'] as String,
      updatedAt: json['updatedAt'] as String,
      group: json['group'] == null ? null : json['group'] as String,
      displayOrder: json['displayOrder'] as int,
      characterCardName: json['characterCardName'] == null ? null : json['characterCardName'] as String,
      characterGroupId: json['characterGroupId'] == null ? null : json['characterGroupId'] as String,
      locked: json['locked'] as bool,
      pinned: json['pinned'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'title': title,
      'updatedAt': updatedAt,
      'group': group,
      'displayOrder': displayOrder,
      'characterCardName': characterCardName,
      'characterGroupId': characterGroupId,
      'locked': locked,
      'pinned': pinned,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final String updatedAt;
  /// Rust field `group` serialized as `group`.
  final String? group;
  /// Rust field `displayOrder` serialized as `displayOrder`.
  final int displayOrder;
  /// Rust field `characterCardName` serialized as `characterCardName`.
  final String? characterCardName;
  /// Rust field `characterGroupId` serialized as `characterGroupId`.
  final String? characterGroupId;
  /// Rust field `locked` serialized as `locked`.
  final bool locked;
  /// Rust field `pinned` serialized as `pinned`.
  final bool pinned;
}

/// Generated Dart model for Rust type `operit_model::ChatMainState::ChatMainState`.
class ChatMainState {
  const ChatMainState({
    required this.currentChatId,
    required this.currentChatTitle,
    required this.currentModelName,
    required this.currentCharacterCardName,
    required this.currentCharacterCardAvatarUri,
    required this.currentWorkspacePath,
    required this.activeCharacterCardName,
    required this.isLoading,
    required this.inputProcessingState,
    required this.messages,
    required this.hasOlderDisplayHistory,
    required this.hasNewerDisplayHistory,
    required this.isLoadingDisplayWindow,
    required this.pendingQueueMessages,
    required this.isPendingQueueExpanded,
  });

  factory ChatMainState.fromJson(Map<String, Object?> json) {
    return ChatMainState(
      currentChatId: json['currentChatId'] == null ? null : json['currentChatId'] as String,
      currentChatTitle: json['currentChatTitle'] as String,
      currentModelName: json['currentModelName'] as String,
      currentCharacterCardName: json['currentCharacterCardName'] == null ? null : json['currentCharacterCardName'] as String,
      currentCharacterCardAvatarUri: json['currentCharacterCardAvatarUri'] == null ? null : json['currentCharacterCardAvatarUri'] as String,
      currentWorkspacePath: json['currentWorkspacePath'] == null ? null : json['currentWorkspacePath'] as String,
      activeCharacterCardName: json['activeCharacterCardName'] == null ? null : json['activeCharacterCardName'] as String,
      isLoading: json['isLoading'] as bool,
      inputProcessingState: json['inputProcessingState'],
      messages: (json['messages'] as List<Object?>).map((item) => item).toList(growable: false),
      hasOlderDisplayHistory: json['hasOlderDisplayHistory'] as bool,
      hasNewerDisplayHistory: json['hasNewerDisplayHistory'] as bool,
      isLoadingDisplayWindow: json['isLoadingDisplayWindow'] as bool,
      pendingQueueMessages: (json['pendingQueueMessages'] as List<Object?>).map((item) => PendingQueueMessageItem.fromJson(item as Map<String, Object?>)).toList(growable: false),
      isPendingQueueExpanded: json['isPendingQueueExpanded'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'currentChatId': currentChatId,
      'currentChatTitle': currentChatTitle,
      'currentModelName': currentModelName,
      'currentCharacterCardName': currentCharacterCardName,
      'currentCharacterCardAvatarUri': currentCharacterCardAvatarUri,
      'currentWorkspacePath': currentWorkspacePath,
      'activeCharacterCardName': activeCharacterCardName,
      'isLoading': isLoading,
      'inputProcessingState': inputProcessingState,
      'messages': messages.map((item) => item).toList(growable: false),
      'hasOlderDisplayHistory': hasOlderDisplayHistory,
      'hasNewerDisplayHistory': hasNewerDisplayHistory,
      'isLoadingDisplayWindow': isLoadingDisplayWindow,
      'pendingQueueMessages': pendingQueueMessages.map((item) => item.toJson()).toList(growable: false),
      'isPendingQueueExpanded': isPendingQueueExpanded,
    };
  }

  /// Rust field `currentChatId` serialized as `currentChatId`.
  final String? currentChatId;
  /// Rust field `currentChatTitle` serialized as `currentChatTitle`.
  final String currentChatTitle;
  /// Rust field `currentModelName` serialized as `currentModelName`.
  final String currentModelName;
  /// Rust field `currentCharacterCardName` serialized as `currentCharacterCardName`.
  final String? currentCharacterCardName;
  /// Rust field `currentCharacterCardAvatarUri` serialized as `currentCharacterCardAvatarUri`.
  final String? currentCharacterCardAvatarUri;
  /// Rust field `currentWorkspacePath` serialized as `currentWorkspacePath`.
  final String? currentWorkspacePath;
  /// Rust field `activeCharacterCardName` serialized as `activeCharacterCardName`.
  final String? activeCharacterCardName;
  /// Rust field `isLoading` serialized as `isLoading`.
  final bool isLoading;
  /// Rust field `inputProcessingState` serialized as `inputProcessingState`.
  final Object? inputProcessingState;
  /// Rust field `messages` serialized as `messages`.
  final List<Object?> messages;
  /// Rust field `hasOlderDisplayHistory` serialized as `hasOlderDisplayHistory`.
  final bool hasOlderDisplayHistory;
  /// Rust field `hasNewerDisplayHistory` serialized as `hasNewerDisplayHistory`.
  final bool hasNewerDisplayHistory;
  /// Rust field `isLoadingDisplayWindow` serialized as `isLoadingDisplayWindow`.
  final bool isLoadingDisplayWindow;
  /// Rust field `pendingQueueMessages` serialized as `pendingQueueMessages`.
  final List<PendingQueueMessageItem> pendingQueueMessages;
  /// Rust field `isPendingQueueExpanded` serialized as `isPendingQueueExpanded`.
  final bool isPendingQueueExpanded;
}

/// Generated Dart model for Rust type `operit_model::ChatMessage::ChatMessage`.
class ChatMessage {
  const ChatMessage({
    required this.sender,
    required this.parts,
    required this.timestamp,
    required this.roleName,
    required this.selectedVariantIndex,
    required this.variantCount,
    required this.provider,
    required this.modelName,
    required this.inputTokens,
    required this.outputTokens,
    required this.cachedInputTokens,
    required this.sentAt,
    required this.outputDurationMs,
    required this.waitDurationMs,
    required this.completedAt,
    required this.displayMode,
    required this.isFavorite,
    required this.isVariantPreview,
  });

  factory ChatMessage.fromJson(Map<String, Object?> json) {
    return ChatMessage(
      sender: json['sender'] as String,
      parts: (json['parts'] as List<Object?>).map((item) => MessagePart.fromJson(item as Map<String, Object?>)).toList(growable: false),
      timestamp: json['timestamp'] as int,
      roleName: json['roleName'] as String,
      selectedVariantIndex: json['selectedVariantIndex'] as int,
      variantCount: json['variantCount'] as int,
      provider: json['provider'] as String,
      modelName: json['modelName'] as String,
      inputTokens: json['inputTokens'] as int,
      outputTokens: json['outputTokens'] as int,
      cachedInputTokens: json['cachedInputTokens'] as int,
      sentAt: json['sentAt'] as int,
      outputDurationMs: json['outputDurationMs'] as int,
      waitDurationMs: json['waitDurationMs'] as int,
      completedAt: json['completedAt'] as int,
      displayMode: ChatMessageDisplayMode.fromJson(json['displayMode']),
      isFavorite: json['isFavorite'] as bool,
      isVariantPreview: json['isVariantPreview'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sender': sender,
      'parts': parts.map((item) => item.toJson()).toList(growable: false),
      'timestamp': timestamp,
      'roleName': roleName,
      'selectedVariantIndex': selectedVariantIndex,
      'variantCount': variantCount,
      'provider': provider,
      'modelName': modelName,
      'inputTokens': inputTokens,
      'outputTokens': outputTokens,
      'cachedInputTokens': cachedInputTokens,
      'sentAt': sentAt,
      'outputDurationMs': outputDurationMs,
      'waitDurationMs': waitDurationMs,
      'completedAt': completedAt,
      'displayMode': displayMode.toJson(),
      'isFavorite': isFavorite,
      'isVariantPreview': isVariantPreview,
    };
  }

  /// Rust field `sender` serialized as `sender`.
  final String sender;
  /// Rust field `parts` serialized as `parts`.
  final List<MessagePart> parts;
  /// Rust field `timestamp` serialized as `timestamp`.
  final int timestamp;
  /// Rust field `roleName` serialized as `roleName`.
  final String roleName;
  /// Rust field `selectedVariantIndex` serialized as `selectedVariantIndex`.
  final int selectedVariantIndex;
  /// Rust field `variantCount` serialized as `variantCount`.
  final int variantCount;
  /// Rust field `provider` serialized as `provider`.
  final String provider;
  /// Rust field `modelName` serialized as `modelName`.
  final String modelName;
  /// Rust field `inputTokens` serialized as `inputTokens`.
  final int inputTokens;
  /// Rust field `outputTokens` serialized as `outputTokens`.
  final int outputTokens;
  /// Rust field `cachedInputTokens` serialized as `cachedInputTokens`.
  final int cachedInputTokens;
  /// Rust field `sentAt` serialized as `sentAt`.
  final int sentAt;
  /// Rust field `outputDurationMs` serialized as `outputDurationMs`.
  final int outputDurationMs;
  /// Rust field `waitDurationMs` serialized as `waitDurationMs`.
  final int waitDurationMs;
  /// Rust field `completedAt` serialized as `completedAt`.
  final int completedAt;
  /// Rust field `displayMode` serialized as `displayMode`.
  final ChatMessageDisplayMode displayMode;
  /// Rust field `isFavorite` serialized as `isFavorite`.
  final bool isFavorite;
  /// Rust field `isVariantPreview` serialized as `isVariantPreview`.
  final bool isVariantPreview;
}

/// Generated Dart enum for Rust type `operit_model::ChatMessageDisplayMode::ChatMessageDisplayMode`.
enum ChatMessageDisplayMode {
  normal('NORMAL'),
  hiddenPlaceholder('HIDDEN_PLACEHOLDER'),
  ;

  const ChatMessageDisplayMode(this.value);

  final String value;

  factory ChatMessageDisplayMode.fromJson(Object? value) {
    return switch (value) {
      'NORMAL' => ChatMessageDisplayMode.normal,
      'HIDDEN_PLACEHOLDER' => ChatMessageDisplayMode.hiddenPlaceholder,
      _ => throw ArgumentError('Unknown ChatMessageDisplayMode: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_model::ChatMessageLocatorPreview::ChatMessageLocatorPreview`.
class ChatMessageLocatorPreview {
  const ChatMessageLocatorPreview({
    required this.messageIndex,
    required this.timestamp,
    required this.sender,
    required this.previewContent,
    required this.contentLength,
    required this.displayMode,
    required this.isFavorite,
  });

  factory ChatMessageLocatorPreview.fromJson(Map<String, Object?> json) {
    return ChatMessageLocatorPreview(
      messageIndex: json['messageIndex'] == null ? null : json['messageIndex'] as int,
      timestamp: json['timestamp'] as int,
      sender: json['sender'] as String,
      previewContent: json['previewContent'] as String,
      contentLength: json['contentLength'] as int,
      displayMode: json['displayMode'] as String,
      isFavorite: json['isFavorite'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'messageIndex': messageIndex,
      'timestamp': timestamp,
      'sender': sender,
      'previewContent': previewContent,
      'contentLength': contentLength,
      'displayMode': displayMode,
      'isFavorite': isFavorite,
    };
  }

  /// Rust field `messageIndex` serialized as `messageIndex`.
  final int? messageIndex;
  /// Rust field `timestamp` serialized as `timestamp`.
  final int timestamp;
  /// Rust field `sender` serialized as `sender`.
  final String sender;
  /// Rust field `previewContent` serialized as `previewContent`.
  final String previewContent;
  /// Rust field `contentLength` serialized as `contentLength`.
  final int contentLength;
  /// Rust field `displayMode` serialized as `displayMode`.
  final String displayMode;
  /// Rust field `isFavorite` serialized as `isFavorite`.
  final bool isFavorite;
}

/// Generated Dart model for Rust type `operit_model::ChatTurnOptions::ChatTurnOptions`.
class ChatTurnOptions {
  const ChatTurnOptions({
    required this.persistTurn,
    required this.notifyReply,
    required this.hideUserMessage,
    required this.disableWarning,
    required this.chatInputSubmitRequestedHandled,
  });

  factory ChatTurnOptions.fromJson(Map<String, Object?> json) {
    return ChatTurnOptions(
      persistTurn: json['persistTurn'] as bool,
      notifyReply: json['notifyReply'] == null ? null : json['notifyReply'] as bool,
      hideUserMessage: json['hideUserMessage'] as bool,
      disableWarning: json['disableWarning'] as bool,
      chatInputSubmitRequestedHandled: json['chatInputSubmitRequestedHandled'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'persistTurn': persistTurn,
      'notifyReply': notifyReply,
      'hideUserMessage': hideUserMessage,
      'disableWarning': disableWarning,
      'chatInputSubmitRequestedHandled': chatInputSubmitRequestedHandled,
    };
  }

  /// Rust field `persistTurn` serialized as `persistTurn`.
  final bool persistTurn;
  /// Rust field `notifyReply` serialized as `notifyReply`.
  final bool? notifyReply;
  /// Rust field `hideUserMessage` serialized as `hideUserMessage`.
  final bool hideUserMessage;
  /// Rust field `disableWarning` serialized as `disableWarning`.
  final bool disableWarning;
  /// Rust field `chatInputSubmitRequestedHandled` serialized as `chatInputSubmitRequestedHandled`.
  final bool chatInputSubmitRequestedHandled;
}

/// Generated Dart enum for Rust type `operit_model::FunctionType::FunctionType`.
enum FunctionType {
  chat('CHAT'),
  summary('SUMMARY'),
  memory('MEMORY'),
  uiController('UI_CONTROLLER'),
  translation('TRANSLATION'),
  grep('GREP'),
  roleResponsePlanner('ROLE_RESPONSE_PLANNER'),
  imageRecognition('IMAGE_RECOGNITION'),
  audioRecognition('AUDIO_RECOGNITION'),
  videoRecognition('VIDEO_RECOGNITION'),
  ;

  const FunctionType(this.value);

  final String value;

  factory FunctionType.fromJson(Object? value) {
    return switch (value) {
      'CHAT' => FunctionType.chat,
      'SUMMARY' => FunctionType.summary,
      'MEMORY' => FunctionType.memory,
      'UI_CONTROLLER' => FunctionType.uiController,
      'TRANSLATION' => FunctionType.translation,
      'GREP' => FunctionType.grep,
      'ROLE_RESPONSE_PLANNER' => FunctionType.roleResponsePlanner,
      'IMAGE_RECOGNITION' => FunctionType.imageRecognition,
      'AUDIO_RECOGNITION' => FunctionType.audioRecognition,
      'VIDEO_RECOGNITION' => FunctionType.videoRecognition,
      _ => throw ArgumentError('Unknown FunctionType: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart tagged enum model for Rust type `operit_model::InputProcessingState::InputProcessingState`.
class InputProcessingState {
  const InputProcessingState._({
    this.message = '',
    this.toolName = '',
    this.progress = 0.0,
    required this.tag,
  });

  factory InputProcessingState.idle() => InputProcessingState._(tag: 'Idle');
  factory InputProcessingState.processing({required String message, }) => InputProcessingState._(tag: 'Processing', message: message);
  factory InputProcessingState.connecting({required String message, }) => InputProcessingState._(tag: 'Connecting', message: message);
  factory InputProcessingState.receiving({required String message, }) => InputProcessingState._(tag: 'Receiving', message: message);
  factory InputProcessingState.executingTool({required String toolName, }) => InputProcessingState._(tag: 'ExecutingTool', toolName: toolName);
  factory InputProcessingState.toolProgress({required String toolName, required double progress, required String message, }) => InputProcessingState._(tag: 'ToolProgress', toolName: toolName, progress: progress, message: message);
  factory InputProcessingState.processingToolResult({required String toolName, }) => InputProcessingState._(tag: 'ProcessingToolResult', toolName: toolName);
  factory InputProcessingState.summarizing({required String message, }) => InputProcessingState._(tag: 'Summarizing', message: message);
  factory InputProcessingState.executingPlan({required String message, }) => InputProcessingState._(tag: 'ExecutingPlan', message: message);
  factory InputProcessingState.completed() => InputProcessingState._(tag: 'Completed');
  factory InputProcessingState.error({required String message, }) => InputProcessingState._(tag: 'Error', message: message);
  final String tag;
  final String message;
  final String toolName;
  final double progress;

  factory InputProcessingState.fromJson(Object? json) {
    switch (json) {
      case 'Idle':
        return InputProcessingState.idle();
      case 'Completed':
        return InputProcessingState.completed();
      case Map<String, Object?> map:
        final tag = map.keys.single;
        final data = map[tag] as Map<String, Object?>;
        return switch (tag) {
          'Processing' => InputProcessingState.processing(message: data['message'] as String, ),
          'Connecting' => InputProcessingState.connecting(message: data['message'] as String, ),
          'Receiving' => InputProcessingState.receiving(message: data['message'] as String, ),
          'ExecutingTool' => InputProcessingState.executingTool(toolName: data['toolName'] as String, ),
          'ToolProgress' => InputProcessingState.toolProgress(toolName: data['toolName'] as String, progress: (data['progress'] as num).toDouble(), message: data['message'] as String, ),
          'ProcessingToolResult' => InputProcessingState.processingToolResult(toolName: data['toolName'] as String, ),
          'Summarizing' => InputProcessingState.summarizing(message: data['message'] as String, ),
          'ExecutingPlan' => InputProcessingState.executingPlan(message: data['message'] as String, ),
          'Error' => InputProcessingState.error(message: data['message'] as String, ),
          _ => throw ArgumentError('Unknown InputProcessingState tag: $tag'),
        };
      default:
        throw ArgumentError('Unknown InputProcessingState representation: $json');
    }
  }

  Object toJson() {
    return switch (tag) {
      'Idle' => 'Idle',
      'Processing' => <String, Object?>{'Processing': <String, Object?>{
        'message': message,
      }},
      'Connecting' => <String, Object?>{'Connecting': <String, Object?>{
        'message': message,
      }},
      'Receiving' => <String, Object?>{'Receiving': <String, Object?>{
        'message': message,
      }},
      'ExecutingTool' => <String, Object?>{'ExecutingTool': <String, Object?>{
        'toolName': toolName,
      }},
      'ToolProgress' => <String, Object?>{'ToolProgress': <String, Object?>{
        'toolName': toolName,
        'progress': progress,
        'message': message,
      }},
      'ProcessingToolResult' => <String, Object?>{'ProcessingToolResult': <String, Object?>{
        'toolName': toolName,
      }},
      'Summarizing' => <String, Object?>{'Summarizing': <String, Object?>{
        'message': message,
      }},
      'ExecutingPlan' => <String, Object?>{'ExecutingPlan': <String, Object?>{
        'message': message,
      }},
      'Completed' => 'Completed',
      'Error' => <String, Object?>{'Error': <String, Object?>{
        'message': message,
      }},
      _ => throw StateError('Unknown InputProcessingState tag: $tag'),
    };
  }
}

/// Generated Dart model for Rust type `operit_model::Memory::Memory`.
class Memory {
  const Memory({
    required this.id,
    required this.uuid,
    required this.title,
    required this.content,
    required this.contentType,
    required this.source,
    required this.credibility,
    required this.importance,
    required this.documentPath,
    required this.isDocumentNode,
    required this.chunkIndexFilePath,
    required this.folderPath,
    required this.createdAt,
    required this.updatedAt,
    required this.lastAccessedAt,
    required this.tags,
    required this.properties,
  });

  factory Memory.fromJson(Map<String, Object?> json) {
    return Memory(
      id: json['id'] as int,
      uuid: json['uuid'] as String,
      title: json['title'] as String,
      content: json['content'] as String,
      contentType: json['contentType'] as String,
      source: json['source'] as String,
      credibility: (json['credibility'] as num).toDouble(),
      importance: (json['importance'] as num).toDouble(),
      documentPath: json['documentPath'] == null ? null : json['documentPath'] as String,
      isDocumentNode: json['isDocumentNode'] as bool,
      chunkIndexFilePath: json['chunkIndexFilePath'] == null ? null : json['chunkIndexFilePath'] as String,
      folderPath: json['folderPath'] == null ? null : json['folderPath'] as String,
      createdAt: json['createdAt'] as int,
      updatedAt: json['updatedAt'] as int,
      lastAccessedAt: json['lastAccessedAt'] as int,
      tags: (json['tags'] as List<Object?>).map((item) => MemoryTag.fromJson(item as Map<String, Object?>)).toList(growable: false),
      properties: (json['properties'] as List<Object?>).map((item) => MemoryProperty.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'uuid': uuid,
      'title': title,
      'content': content,
      'contentType': contentType,
      'source': source,
      'credibility': credibility,
      'importance': importance,
      'documentPath': documentPath,
      'isDocumentNode': isDocumentNode,
      'chunkIndexFilePath': chunkIndexFilePath,
      'folderPath': folderPath,
      'createdAt': createdAt,
      'updatedAt': updatedAt,
      'lastAccessedAt': lastAccessedAt,
      'tags': tags.map((item) => item.toJson()).toList(growable: false),
      'properties': properties.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `id` serialized as `id`.
  final int id;
  /// Rust field `uuid` serialized as `uuid`.
  final String uuid;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `content` serialized as `content`.
  final String content;
  /// Rust field `contentType` serialized as `contentType`.
  final String contentType;
  /// Rust field `source` serialized as `source`.
  final String source;
  /// Rust field `credibility` serialized as `credibility`.
  final double credibility;
  /// Rust field `importance` serialized as `importance`.
  final double importance;
  /// Rust field `documentPath` serialized as `documentPath`.
  final String? documentPath;
  /// Rust field `isDocumentNode` serialized as `isDocumentNode`.
  final bool isDocumentNode;
  /// Rust field `chunkIndexFilePath` serialized as `chunkIndexFilePath`.
  final String? chunkIndexFilePath;
  /// Rust field `folderPath` serialized as `folderPath`.
  final String? folderPath;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
  /// Rust field `lastAccessedAt` serialized as `lastAccessedAt`.
  final int lastAccessedAt;
  /// Rust field `tags` serialized as `tags`.
  final List<MemoryTag> tags;
  /// Rust field `properties` serialized as `properties`.
  final List<MemoryProperty> properties;
}

/// Generated Dart model for Rust type `operit_model::Memory::MemoryGraph`.
class MemoryGraph {
  const MemoryGraph({
    required this.nodes,
    required this.edges,
  });

  factory MemoryGraph.fromJson(Map<String, Object?> json) {
    return MemoryGraph(
      nodes: (json['nodes'] as List<Object?>).map((item) => MemoryGraphNode.fromJson(item as Map<String, Object?>)).toList(growable: false),
      edges: (json['edges'] as List<Object?>).map((item) => MemoryGraphEdge.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'nodes': nodes.map((item) => item.toJson()).toList(growable: false),
      'edges': edges.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `nodes` serialized as `nodes`.
  final List<MemoryGraphNode> nodes;
  /// Rust field `edges` serialized as `edges`.
  final List<MemoryGraphEdge> edges;
}

/// Generated Dart model for Rust type `operit_model::Memory::MemoryGraphEdge`.
class MemoryGraphEdge {
  const MemoryGraphEdge({
    required this.id,
    required this.sourceId,
    required this.targetId,
    required this.label,
    required this.weight,
    required this.metadata,
    required this.isCrossFolderLink,
  });

  factory MemoryGraphEdge.fromJson(Map<String, Object?> json) {
    return MemoryGraphEdge(
      id: json['id'] as int,
      sourceId: json['sourceId'] as String,
      targetId: json['targetId'] as String,
      label: json['label'] == null ? null : json['label'] as String,
      weight: (json['weight'] as num).toDouble(),
      metadata: (json['metadata'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
      isCrossFolderLink: json['isCrossFolderLink'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'sourceId': sourceId,
      'targetId': targetId,
      'label': label,
      'weight': weight,
      'metadata': metadata.map((key, value) => MapEntry(key, value)),
      'isCrossFolderLink': isCrossFolderLink,
    };
  }

  /// Rust field `id` serialized as `id`.
  final int id;
  /// Rust field `sourceId` serialized as `sourceId`.
  final String sourceId;
  /// Rust field `targetId` serialized as `targetId`.
  final String targetId;
  /// Rust field `label` serialized as `label`.
  final String? label;
  /// Rust field `weight` serialized as `weight`.
  final double weight;
  /// Rust field `metadata` serialized as `metadata`.
  final Map<String, String> metadata;
  /// Rust field `isCrossFolderLink` serialized as `isCrossFolderLink`.
  final bool isCrossFolderLink;
}

/// Generated Dart model for Rust type `operit_model::Memory::MemoryGraphNode`.
class MemoryGraphNode {
  const MemoryGraphNode({
    required this.id,
    required this.label,
    required this.color,
    required this.metadata,
  });

  factory MemoryGraphNode.fromJson(Map<String, Object?> json) {
    return MemoryGraphNode(
      id: json['id'] as String,
      label: json['label'] as String,
      color: json['color'] as int,
      metadata: (json['metadata'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'label': label,
      'color': color,
      'metadata': metadata.map((key, value) => MapEntry(key, value)),
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `label` serialized as `label`.
  final String label;
  /// Rust field `color` serialized as `color`.
  final int color;
  /// Rust field `metadata` serialized as `metadata`.
  final Map<String, String> metadata;
}

/// Generated Dart model for Rust type `operit_model::Memory::MemoryLink`.
class MemoryLink {
  const MemoryLink({
    required this.id,
    required this.sourceMemoryId,
    required this.targetMemoryId,
    required this.type,
    required this.weight,
    required this.description,
  });

  factory MemoryLink.fromJson(Map<String, Object?> json) {
    return MemoryLink(
      id: json['id'] as int,
      sourceMemoryId: json['sourceMemoryId'] as int,
      targetMemoryId: json['targetMemoryId'] as int,
      type: json['type_'] as String,
      weight: (json['weight'] as num).toDouble(),
      description: json['description'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'sourceMemoryId': sourceMemoryId,
      'targetMemoryId': targetMemoryId,
      'type_': type,
      'weight': weight,
      'description': description,
    };
  }

  /// Rust field `id` serialized as `id`.
  final int id;
  /// Rust field `sourceMemoryId` serialized as `sourceMemoryId`.
  final int sourceMemoryId;
  /// Rust field `targetMemoryId` serialized as `targetMemoryId`.
  final int targetMemoryId;
  /// Rust field `type_` serialized as `type_`.
  final String type;
  /// Rust field `weight` serialized as `weight`.
  final double weight;
  /// Rust field `description` serialized as `description`.
  final String description;
}

/// Generated Dart model for Rust type `operit_model::Memory::MemoryProperty`.
class MemoryProperty {
  const MemoryProperty({
    required this.id,
    required this.key,
    required this.value,
  });

  factory MemoryProperty.fromJson(Map<String, Object?> json) {
    return MemoryProperty(
      id: json['id'] as int,
      key: json['key'] as String,
      value: json['value'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'key': key,
      'value': value,
    };
  }

  /// Rust field `id` serialized as `id`.
  final int id;
  /// Rust field `key` serialized as `key`.
  final String key;
  /// Rust field `value` serialized as `value`.
  final String value;
}

/// Generated Dart model for Rust type `operit_model::Memory::MemoryTag`.
class MemoryTag {
  const MemoryTag({
    required this.id,
    required this.name,
  });

  factory MemoryTag.fromJson(Map<String, Object?> json) {
    return MemoryTag(
      id: json['id'] as int,
      name: json['name'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
    };
  }

  /// Rust field `id` serialized as `id`.
  final int id;
  /// Rust field `name` serialized as `name`.
  final String name;
}

/// Generated Dart model for Rust type `operit_model::Memory::SharedMemoryStore`.
class SharedMemoryStore {
  const SharedMemoryStore({
    required this.id,
    required this.name,
    required this.createdAt,
    required this.updatedAt,
  });

  factory SharedMemoryStore.fromJson(Map<String, Object?> json) {
    return SharedMemoryStore(
      id: json['id'] as String,
      name: json['name'] as String,
      createdAt: json['createdAt'] as int,
      updatedAt: json['updatedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
      'createdAt': createdAt,
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
}

/// Generated Dart enum for Rust type `operit_model::MemoryExportModel::ImportStrategy`.
enum ImportStrategy {
  skip('SKIP'),
  update('UPDATE'),
  createNew('CREATE_NEW'),
  ;

  const ImportStrategy(this.value);

  final String value;

  factory ImportStrategy.fromJson(Object? value) {
    return switch (value) {
      'SKIP' => ImportStrategy.skip,
      'UPDATE' => ImportStrategy.update,
      'CREATE_NEW' => ImportStrategy.createNew,
      _ => throw ArgumentError('Unknown ImportStrategy: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_model::MemoryExportModel::MemoryImportResult`.
class MemoryImportResult {
  const MemoryImportResult({
    required this.newMemories,
    required this.updatedMemories,
    required this.skippedMemories,
    required this.newLinks,
  });

  factory MemoryImportResult.fromJson(Map<String, Object?> json) {
    return MemoryImportResult(
      newMemories: json['newMemories'] as int,
      updatedMemories: json['updatedMemories'] as int,
      skippedMemories: json['skippedMemories'] as int,
      newLinks: json['newLinks'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'newMemories': newMemories,
      'updatedMemories': updatedMemories,
      'skippedMemories': skippedMemories,
      'newLinks': newLinks,
    };
  }

  /// Rust field `newMemories` serialized as `newMemories`.
  final int newMemories;
  /// Rust field `updatedMemories` serialized as `updatedMemories`.
  final int updatedMemories;
  /// Rust field `skippedMemories` serialized as `skippedMemories`.
  final int skippedMemories;
  /// Rust field `newLinks` serialized as `newLinks`.
  final int newLinks;
}

/// Generated Dart model for Rust type `operit_model::MessagePart::MessagePart`.
class MessagePart {
  const MessagePart({
    required this.partId,
    required this.sequence,
    required this.kind,
    required this.content,
    required this.toolCallId,
    required this.toolName,
    required this.attributes,
  });

  factory MessagePart.fromJson(Map<String, Object?> json) {
    return MessagePart(
      partId: json['partId'] as String,
      sequence: json['sequence'] as int,
      kind: MessagePartKind.fromJson(json['kind']),
      content: json['content'] as String,
      toolCallId: json['toolCallId'] == null ? null : json['toolCallId'] as String,
      toolName: json['toolName'] == null ? null : json['toolName'] as String,
      attributes: (json['attributes'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'partId': partId,
      'sequence': sequence,
      'kind': kind.toJson(),
      'content': content,
      'toolCallId': toolCallId,
      'toolName': toolName,
      'attributes': attributes.map((key, value) => MapEntry(key, value)),
    };
  }

  /// Rust field `partId` serialized as `partId`.
  final String partId;
  /// Rust field `sequence` serialized as `sequence`.
  final int sequence;
  /// Rust field `kind` serialized as `kind`.
  final MessagePartKind kind;
  /// Rust field `content` serialized as `content`.
  final String content;
  /// Rust field `toolCallId` serialized as `toolCallId`.
  final String? toolCallId;
  /// Rust field `toolName` serialized as `toolName`.
  final String? toolName;
  /// Rust field `attributes` serialized as `attributes`.
  final Map<String, String> attributes;
}

/// Generated Dart enum for Rust type `operit_model::MessagePart::MessagePartKind`.
enum MessagePartKind {
  markdown('markdown'),
  thinking('thinking'),
  toolCall('tool_call'),
  toolResult('tool_result'),
  status('status'),
  ;

  const MessagePartKind(this.value);

  final String value;

  factory MessagePartKind.fromJson(Object? value) {
    return switch (value) {
      'markdown' => MessagePartKind.markdown,
      'thinking' => MessagePartKind.thinking,
      'tool_call' => MessagePartKind.toolCall,
      'tool_result' => MessagePartKind.toolResult,
      'status' => MessagePartKind.status,
      _ => throw ArgumentError('Unknown MessagePartKind: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_model::ModelConfigData::ApiProviderType`.
enum ApiProviderType {
  openai('OPENAI'),
  openaiResponses('OPENAI_RESPONSES'),
  openaiResponsesGeneric('OPENAI_RESPONSES_GENERIC'),
  openaiGeneric('OPENAI_GENERIC'),
  anthropic('ANTHROPIC'),
  anthropicGeneric('ANTHROPIC_GENERIC'),
  google('GOOGLE'),
  geminiGeneric('GEMINI_GENERIC'),
  baidu('BAIDU'),
  aliyun('ALIYUN'),
  xunfei('XUNFEI'),
  zhipu('ZHIPU'),
  baichuan('BAICHUAN'),
  moonshot('MOONSHOT'),
  mimo('MIMO'),
  deepseek('DEEPSEEK'),
  mistral('MISTRAL'),
  siliconflow('SILICONFLOW'),
  iflow('IFLOW'),
  openrouter('OPENROUTER'),
  fourRouter('FOUR_ROUTER'),
  nousPortal('NOUS_PORTAL'),
  infiniai('INFINIAI'),
  alipayBailing('ALIPAY_BAILING'),
  doubao('DOUBAO'),
  nvidia('NVIDIA'),
  lmstudio('LMSTUDIO'),
  ollama('OLLAMA'),
  openaiLocal('OPENAI_LOCAL'),
  localModel('LOCAL_MODEL'),
  ppinfra('PPINFRA'),
  novita('NOVITA'),
  other('OTHER'),
  ;

  const ApiProviderType(this.value);

  final String value;

  factory ApiProviderType.fromJson(Object? value) {
    return switch (value) {
      'OPENAI' => ApiProviderType.openai,
      'OPENAI_RESPONSES' => ApiProviderType.openaiResponses,
      'OPENAI_RESPONSES_GENERIC' => ApiProviderType.openaiResponsesGeneric,
      'OPENAI_GENERIC' => ApiProviderType.openaiGeneric,
      'ANTHROPIC' => ApiProviderType.anthropic,
      'ANTHROPIC_GENERIC' => ApiProviderType.anthropicGeneric,
      'GOOGLE' => ApiProviderType.google,
      'GEMINI_GENERIC' => ApiProviderType.geminiGeneric,
      'BAIDU' => ApiProviderType.baidu,
      'ALIYUN' => ApiProviderType.aliyun,
      'XUNFEI' => ApiProviderType.xunfei,
      'ZHIPU' => ApiProviderType.zhipu,
      'BAICHUAN' => ApiProviderType.baichuan,
      'MOONSHOT' => ApiProviderType.moonshot,
      'MIMO' => ApiProviderType.mimo,
      'DEEPSEEK' => ApiProviderType.deepseek,
      'MISTRAL' => ApiProviderType.mistral,
      'SILICONFLOW' => ApiProviderType.siliconflow,
      'IFLOW' => ApiProviderType.iflow,
      'OPENROUTER' => ApiProviderType.openrouter,
      'FOUR_ROUTER' => ApiProviderType.fourRouter,
      'NOUS_PORTAL' => ApiProviderType.nousPortal,
      'INFINIAI' => ApiProviderType.infiniai,
      'ALIPAY_BAILING' => ApiProviderType.alipayBailing,
      'DOUBAO' => ApiProviderType.doubao,
      'NVIDIA' => ApiProviderType.nvidia,
      'LMSTUDIO' => ApiProviderType.lmstudio,
      'OLLAMA' => ApiProviderType.ollama,
      'OPENAI_LOCAL' => ApiProviderType.openaiLocal,
      'LOCAL_MODEL' => ApiProviderType.localModel,
      'PPINFRA' => ApiProviderType.ppinfra,
      'NOVITA' => ApiProviderType.novita,
      'OTHER' => ApiProviderType.other,
      _ => throw ArgumentError('Unknown ApiProviderType: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::AvailableProviderModel`.
class AvailableProviderModel {
  const AvailableProviderModel({
    required this.modelId,
    required this.source,
    required this.pricing,
    required this.context,
    required this.capabilities,
    required this.builtinTools,
    required this.request,
  });

  factory AvailableProviderModel.fromJson(Map<String, Object?> json) {
    return AvailableProviderModel(
      modelId: json['modelId'] as String,
      source: AvailableProviderModelSource.fromJson(json['source']),
      pricing: json['pricing'] == null ? null : ModelPricing.fromJson(json['pricing'] as Map<String, Object?>),
      context: json['context'] == null ? null : ModelContextSpec.fromJson(json['context'] as Map<String, Object?>),
      capabilities: json['capabilities'] == null ? null : ModelCapabilities.fromJson(json['capabilities'] as Map<String, Object?>),
      builtinTools: (json['builtinTools'] as List<Object?>).map((item) => ModelBuiltinTool.fromJson(item as Map<String, Object?>)).toList(growable: false),
      request: json['request'] == null ? null : ModelRequestSpec.fromJson(json['request'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'modelId': modelId,
      'source': source.toJson(),
      'pricing': pricing?.toJson(),
      'context': context?.toJson(),
      'capabilities': capabilities?.toJson(),
      'builtinTools': builtinTools.map((item) => item.toJson()).toList(growable: false),
      'request': request?.toJson(),
    };
  }

  /// Rust field `modelId` serialized as `modelId`.
  final String modelId;
  /// Rust field `source` serialized as `source`.
  final AvailableProviderModelSource source;
  /// Rust field `pricing` serialized as `pricing`.
  final ModelPricing? pricing;
  /// Rust field `context` serialized as `context`.
  final ModelContextSpec? context;
  /// Rust field `capabilities` serialized as `capabilities`.
  final ModelCapabilities? capabilities;
  /// Rust field `builtinTools` serialized as `builtinTools`.
  final List<ModelBuiltinTool> builtinTools;
  /// Rust field `request` serialized as `request`.
  final ModelRequestSpec? request;
}

/// Generated Dart enum for Rust type `operit_model::ModelConfigData::AvailableProviderModelSource`.
enum AvailableProviderModelSource {
  catalog('Catalog'),
  remote('Remote'),
  local('Local'),
  ;

  const AvailableProviderModelSource(this.value);

  final String value;

  factory AvailableProviderModelSource.fromJson(Object? value) {
    return switch (value) {
      'Catalog' => AvailableProviderModelSource.catalog,
      'Remote' => AvailableProviderModelSource.remote,
      'Local' => AvailableProviderModelSource.local,
      _ => throw ArgumentError('Unknown AvailableProviderModelSource: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_model::ModelConfigData::BuiltinToolExclusivity`.
enum BuiltinToolExclusivity {
  canMixWithExternalTools('CanMixWithExternalTools'),
  exclusiveWithExternalTools('ExclusiveWithExternalTools'),
  ;

  const BuiltinToolExclusivity(this.value);

  final String value;

  factory BuiltinToolExclusivity.fromJson(Object? value) {
    return switch (value) {
      'CanMixWithExternalTools' => BuiltinToolExclusivity.canMixWithExternalTools,
      'ExclusiveWithExternalTools' => BuiltinToolExclusivity.exclusiveWithExternalTools,
      _ => throw ArgumentError('Unknown BuiltinToolExclusivity: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_model::ModelConfigData::BuiltinToolRequestFormat`.
enum BuiltinToolRequestFormat {
  geminiGoogleSearch('GeminiGoogleSearch'),
  anthropicWebSearch('AnthropicWebSearch'),
  openAiWebSearch('OpenAiWebSearch'),
  ;

  const BuiltinToolRequestFormat(this.value);

  final String value;

  factory BuiltinToolRequestFormat.fromJson(Object? value) {
    return switch (value) {
      'GeminiGoogleSearch' => BuiltinToolRequestFormat.geminiGoogleSearch,
      'AnthropicWebSearch' => BuiltinToolRequestFormat.anthropicWebSearch,
      'OpenAiWebSearch' => BuiltinToolRequestFormat.openAiWebSearch,
      _ => throw ArgumentError('Unknown BuiltinToolRequestFormat: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_model::ModelConfigData::BuiltinToolType`.
enum BuiltinToolType {
  webSearch('WebSearch'),
  codeExecution('CodeExecution'),
  urlContext('UrlContext'),
  ;

  const BuiltinToolType(this.value);

  final String value;

  factory BuiltinToolType.fromJson(Object? value) {
    return switch (value) {
      'WebSearch' => BuiltinToolType.webSearch,
      'CodeExecution' => BuiltinToolType.codeExecution,
      'UrlContext' => BuiltinToolType.urlContext,
      _ => throw ArgumentError('Unknown BuiltinToolType: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::LocalModelRuntimeSettings`.
class LocalModelRuntimeSettings {
  const LocalModelRuntimeSettings({
    required this.mnnForwardType,
    required this.mnnThreadCount,
    required this.llamaThreadCount,
    required this.llamaContextSize,
    required this.llamaBatchSize,
    required this.llamaUBatchSize,
    required this.llamaGpuLayers,
    required this.llamaUseMmap,
    required this.llamaFlashAttention,
    required this.llamaKvUnified,
    required this.llamaOffloadKqv,
  });

  factory LocalModelRuntimeSettings.fromJson(Map<String, Object?> json) {
    return LocalModelRuntimeSettings(
      mnnForwardType: json['mnnForwardType'] as int,
      mnnThreadCount: json['mnnThreadCount'] as int,
      llamaThreadCount: json['llamaThreadCount'] as int,
      llamaContextSize: json['llamaContextSize'] as int,
      llamaBatchSize: json['llamaBatchSize'] as int,
      llamaUBatchSize: json['llamaUBatchSize'] as int,
      llamaGpuLayers: json['llamaGpuLayers'] as int,
      llamaUseMmap: json['llamaUseMmap'] as bool,
      llamaFlashAttention: json['llamaFlashAttention'] as bool,
      llamaKvUnified: json['llamaKvUnified'] as bool,
      llamaOffloadKqv: json['llamaOffloadKqv'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'mnnForwardType': mnnForwardType,
      'mnnThreadCount': mnnThreadCount,
      'llamaThreadCount': llamaThreadCount,
      'llamaContextSize': llamaContextSize,
      'llamaBatchSize': llamaBatchSize,
      'llamaUBatchSize': llamaUBatchSize,
      'llamaGpuLayers': llamaGpuLayers,
      'llamaUseMmap': llamaUseMmap,
      'llamaFlashAttention': llamaFlashAttention,
      'llamaKvUnified': llamaKvUnified,
      'llamaOffloadKqv': llamaOffloadKqv,
    };
  }

  /// Rust field `mnnForwardType` serialized as `mnnForwardType`.
  final int mnnForwardType;
  /// Rust field `mnnThreadCount` serialized as `mnnThreadCount`.
  final int mnnThreadCount;
  /// Rust field `llamaThreadCount` serialized as `llamaThreadCount`.
  final int llamaThreadCount;
  /// Rust field `llamaContextSize` serialized as `llamaContextSize`.
  final int llamaContextSize;
  /// Rust field `llamaBatchSize` serialized as `llamaBatchSize`.
  final int llamaBatchSize;
  /// Rust field `llamaUBatchSize` serialized as `llamaUBatchSize`.
  final int llamaUBatchSize;
  /// Rust field `llamaGpuLayers` serialized as `llamaGpuLayers`.
  final int llamaGpuLayers;
  /// Rust field `llamaUseMmap` serialized as `llamaUseMmap`.
  final bool llamaUseMmap;
  /// Rust field `llamaFlashAttention` serialized as `llamaFlashAttention`.
  final bool llamaFlashAttention;
  /// Rust field `llamaKvUnified` serialized as `llamaKvUnified`.
  final bool llamaKvUnified;
  /// Rust field `llamaOffloadKqv` serialized as `llamaOffloadKqv`.
  final bool llamaOffloadKqv;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ModelBuiltinTool`.
class ModelBuiltinTool {
  const ModelBuiltinTool({
    required this.toolType,
    required this.displayName,
    required this.enabled,
    required this.requestFormat,
    required this.exclusivity,
    required this.config,
  });

  factory ModelBuiltinTool.fromJson(Map<String, Object?> json) {
    return ModelBuiltinTool(
      toolType: BuiltinToolType.fromJson(json['toolType']),
      displayName: json['displayName'] as String,
      enabled: json['enabled'] as bool,
      requestFormat: BuiltinToolRequestFormat.fromJson(json['requestFormat']),
      exclusivity: BuiltinToolExclusivity.fromJson(json['exclusivity']),
      config: json['config'],
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'toolType': toolType.toJson(),
      'displayName': displayName,
      'enabled': enabled,
      'requestFormat': requestFormat.toJson(),
      'exclusivity': exclusivity.toJson(),
      'config': config,
    };
  }

  /// Rust field `toolType` serialized as `toolType`.
  final BuiltinToolType toolType;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `enabled` serialized as `enabled`.
  final bool enabled;
  /// Rust field `requestFormat` serialized as `requestFormat`.
  final BuiltinToolRequestFormat requestFormat;
  /// Rust field `exclusivity` serialized as `exclusivity`.
  final BuiltinToolExclusivity exclusivity;
  /// Rust field `config` serialized as `config`.
  final Object? config;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ModelCapabilities`.
class ModelCapabilities {
  const ModelCapabilities({
    required this.directImage,
    required this.directAudio,
    required this.directVideo,
    required this.toolCall,
  });

  factory ModelCapabilities.fromJson(Map<String, Object?> json) {
    return ModelCapabilities(
      directImage: json['directImage'] as bool,
      directAudio: json['directAudio'] as bool,
      directVideo: json['directVideo'] as bool,
      toolCall: json['toolCall'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'directImage': directImage,
      'directAudio': directAudio,
      'directVideo': directVideo,
      'toolCall': toolCall,
    };
  }

  /// Rust field `directImage` serialized as `directImage`.
  final bool directImage;
  /// Rust field `directAudio` serialized as `directAudio`.
  final bool directAudio;
  /// Rust field `directVideo` serialized as `directVideo`.
  final bool directVideo;
  /// Rust field `toolCall` serialized as `toolCall`.
  final bool toolCall;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ModelCatalogEntry`.
class ModelCatalogEntry {
  const ModelCatalogEntry({
    required this.providerTypeId,
    required this.modelId,
    required this.aliases,
    required this.pricing,
    required this.context,
    required this.capabilities,
    required this.builtinTools,
    required this.request,
  });

  factory ModelCatalogEntry.fromJson(Map<String, Object?> json) {
    return ModelCatalogEntry(
      providerTypeId: json['providerTypeId'] as String,
      modelId: json['modelId'] as String,
      aliases: (json['aliases'] as List<Object?>).map((item) => item as String).toList(growable: false),
      pricing: json['pricing'] == null ? null : ModelPricing.fromJson(json['pricing'] as Map<String, Object?>),
      context: json['context'] == null ? null : ModelContextSpec.fromJson(json['context'] as Map<String, Object?>),
      capabilities: json['capabilities'] == null ? null : ModelCapabilities.fromJson(json['capabilities'] as Map<String, Object?>),
      builtinTools: (json['builtinTools'] as List<Object?>).map((item) => ModelBuiltinTool.fromJson(item as Map<String, Object?>)).toList(growable: false),
      request: json['request'] == null ? null : ModelRequestSpec.fromJson(json['request'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'providerTypeId': providerTypeId,
      'modelId': modelId,
      'aliases': aliases.map((item) => item).toList(growable: false),
      'pricing': pricing?.toJson(),
      'context': context?.toJson(),
      'capabilities': capabilities?.toJson(),
      'builtinTools': builtinTools.map((item) => item.toJson()).toList(growable: false),
      'request': request?.toJson(),
    };
  }

  /// Rust field `providerTypeId` serialized as `providerTypeId`.
  final String providerTypeId;
  /// Rust field `modelId` serialized as `modelId`.
  final String modelId;
  /// Rust field `aliases` serialized as `aliases`.
  final List<String> aliases;
  /// Rust field `pricing` serialized as `pricing`.
  final ModelPricing? pricing;
  /// Rust field `context` serialized as `context`.
  final ModelContextSpec? context;
  /// Rust field `capabilities` serialized as `capabilities`.
  final ModelCapabilities? capabilities;
  /// Rust field `builtinTools` serialized as `builtinTools`.
  final List<ModelBuiltinTool> builtinTools;
  /// Rust field `request` serialized as `request`.
  final ModelRequestSpec? request;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ModelCatalogKey`.
class ModelCatalogKey {
  const ModelCatalogKey({
    required this.providerTypeId,
    required this.modelId,
  });

  factory ModelCatalogKey.fromJson(Map<String, Object?> json) {
    return ModelCatalogKey(
      providerTypeId: json['providerTypeId'] as String,
      modelId: json['modelId'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'providerTypeId': providerTypeId,
      'modelId': modelId,
    };
  }

  /// Rust field `providerTypeId` serialized as `providerTypeId`.
  final String providerTypeId;
  /// Rust field `modelId` serialized as `modelId`.
  final String modelId;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ModelContextSpec`.
class ModelContextSpec {
  const ModelContextSpec({
    required this.maxContextLength,
    required this.enableMaxContextMode,
  });

  factory ModelContextSpec.fromJson(Map<String, Object?> json) {
    return ModelContextSpec(
      maxContextLength: (json['maxContextLength'] as num).toDouble(),
      enableMaxContextMode: json['enableMaxContextMode'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'maxContextLength': maxContextLength,
      'enableMaxContextMode': enableMaxContextMode,
    };
  }

  /// Rust field `maxContextLength` serialized as `maxContextLength`.
  final double maxContextLength;
  /// Rust field `enableMaxContextMode` serialized as `enableMaxContextMode`.
  final bool enableMaxContextMode;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ModelPricing`.
class ModelPricing {
  const ModelPricing({
    required this.billingMode,
    required this.inputPricePerMillion,
    required this.cachedInputPricePerMillion,
    required this.outputPricePerMillion,
    required this.pricePerRequest,
    required this.currency,
  });

  factory ModelPricing.fromJson(Map<String, Object?> json) {
    return ModelPricing(
      billingMode: json['billingMode'],
      inputPricePerMillion: (json['inputPricePerMillion'] as num).toDouble(),
      cachedInputPricePerMillion: json['cachedInputPricePerMillion'] == null ? null : (json['cachedInputPricePerMillion'] as num).toDouble(),
      outputPricePerMillion: (json['outputPricePerMillion'] as num).toDouble(),
      pricePerRequest: (json['pricePerRequest'] as num).toDouble(),
      currency: PricingCurrency.fromJson(json['currency']),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'billingMode': billingMode,
      'inputPricePerMillion': inputPricePerMillion,
      'cachedInputPricePerMillion': cachedInputPricePerMillion,
      'outputPricePerMillion': outputPricePerMillion,
      'pricePerRequest': pricePerRequest,
      'currency': currency.toJson(),
    };
  }

  /// Rust field `billingMode` serialized as `billingMode`.
  final Object? billingMode;
  /// Rust field `inputPricePerMillion` serialized as `inputPricePerMillion`.
  final double inputPricePerMillion;
  /// Rust field `cachedInputPricePerMillion` serialized as `cachedInputPricePerMillion`.
  final double? cachedInputPricePerMillion;
  /// Rust field `outputPricePerMillion` serialized as `outputPricePerMillion`.
  final double outputPricePerMillion;
  /// Rust field `pricePerRequest` serialized as `pricePerRequest`.
  final double pricePerRequest;
  /// Rust field `currency` serialized as `currency`.
  final PricingCurrency currency;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ModelProfile`.
class ModelProfile {
  const ModelProfile({
    required this.id,
    required this.catalogKey,
    required this.pricingOverride,
    required this.contextOverride,
    required this.capabilitiesOverride,
    required this.builtinToolsOverride,
    required this.requestOverride,
    required this.parameters,
    required this.summary,
    required this.localRuntime,
  });

  factory ModelProfile.fromJson(Map<String, Object?> json) {
    return ModelProfile(
      id: json['id'] as String,
      catalogKey: json['catalogKey'] == null ? null : ModelCatalogKey.fromJson(json['catalogKey'] as Map<String, Object?>),
      pricingOverride: json['pricingOverride'] == null ? null : ModelPricing.fromJson(json['pricingOverride'] as Map<String, Object?>),
      contextOverride: json['contextOverride'] == null ? null : ModelContextSpec.fromJson(json['contextOverride'] as Map<String, Object?>),
      capabilitiesOverride: json['capabilitiesOverride'] == null ? null : ModelCapabilities.fromJson(json['capabilitiesOverride'] as Map<String, Object?>),
      builtinToolsOverride: json['builtinToolsOverride'] == null ? null : (json['builtinToolsOverride'] as List<Object?>).map((item) => ModelBuiltinTool.fromJson(item as Map<String, Object?>)).toList(growable: false),
      requestOverride: json['requestOverride'] == null ? null : ModelRequestSpec.fromJson(json['requestOverride'] as Map<String, Object?>),
      parameters: (json['parameters'] as List<Object?>).map((item) => item).toList(growable: false),
      summary: ModelSummarySettings.fromJson(json['summary'] as Map<String, Object?>),
      localRuntime: LocalModelRuntimeSettings.fromJson(json['localRuntime'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'catalogKey': catalogKey?.toJson(),
      'pricingOverride': pricingOverride?.toJson(),
      'contextOverride': contextOverride?.toJson(),
      'capabilitiesOverride': capabilitiesOverride?.toJson(),
      'builtinToolsOverride': builtinToolsOverride?.map((item) => item.toJson()).toList(growable: false),
      'requestOverride': requestOverride?.toJson(),
      'parameters': parameters.map((item) => item).toList(growable: false),
      'summary': summary.toJson(),
      'localRuntime': localRuntime.toJson(),
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `catalogKey` serialized as `catalogKey`.
  final ModelCatalogKey? catalogKey;
  /// Rust field `pricingOverride` serialized as `pricingOverride`.
  final ModelPricing? pricingOverride;
  /// Rust field `contextOverride` serialized as `contextOverride`.
  final ModelContextSpec? contextOverride;
  /// Rust field `capabilitiesOverride` serialized as `capabilitiesOverride`.
  final ModelCapabilities? capabilitiesOverride;
  /// Rust field `builtinToolsOverride` serialized as `builtinToolsOverride`.
  final List<ModelBuiltinTool>? builtinToolsOverride;
  /// Rust field `requestOverride` serialized as `requestOverride`.
  final ModelRequestSpec? requestOverride;
  /// Rust field `parameters` serialized as `parameters`.
  final List<Object?> parameters;
  /// Rust field `summary` serialized as `summary`.
  final ModelSummarySettings summary;
  /// Rust field `localRuntime` serialized as `localRuntime`.
  final LocalModelRuntimeSettings localRuntime;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ModelRequestSpec`.
class ModelRequestSpec {
  const ModelRequestSpec({
    required this.supportsStructuredTools,
  });

  factory ModelRequestSpec.fromJson(Map<String, Object?> json) {
    return ModelRequestSpec(
      supportsStructuredTools: json['supportsStructuredTools'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'supportsStructuredTools': supportsStructuredTools,
    };
  }

  /// Rust field `supportsStructuredTools` serialized as `supportsStructuredTools`.
  final bool supportsStructuredTools;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ModelSummarySettings`.
class ModelSummarySettings {
  const ModelSummarySettings({
    required this.enableSummary,
    required this.summaryTokenThreshold,
    required this.enableSummaryByMessageCount,
    required this.summaryMessageCountThreshold,
  });

  factory ModelSummarySettings.fromJson(Map<String, Object?> json) {
    return ModelSummarySettings(
      enableSummary: json['enableSummary'] as bool,
      summaryTokenThreshold: (json['summaryTokenThreshold'] as num).toDouble(),
      enableSummaryByMessageCount: json['enableSummaryByMessageCount'] as bool,
      summaryMessageCountThreshold: json['summaryMessageCountThreshold'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'enableSummary': enableSummary,
      'summaryTokenThreshold': summaryTokenThreshold,
      'enableSummaryByMessageCount': enableSummaryByMessageCount,
      'summaryMessageCountThreshold': summaryMessageCountThreshold,
    };
  }

  /// Rust field `enableSummary` serialized as `enableSummary`.
  final bool enableSummary;
  /// Rust field `summaryTokenThreshold` serialized as `summaryTokenThreshold`.
  final double summaryTokenThreshold;
  /// Rust field `enableSummaryByMessageCount` serialized as `enableSummaryByMessageCount`.
  final bool enableSummaryByMessageCount;
  /// Rust field `summaryMessageCountThreshold` serialized as `summaryMessageCountThreshold`.
  final int summaryMessageCountThreshold;
}

/// Generated Dart enum for Rust type `operit_model::ModelConfigData::PricingCurrency`.
enum PricingCurrency {
  cny('CNY'),
  usd('USD'),
  ;

  const PricingCurrency(this.value);

  final String value;

  factory PricingCurrency.fromJson(Object? value) {
    return switch (value) {
      'CNY' => PricingCurrency.cny,
      'USD' => PricingCurrency.usd,
      _ => throw ArgumentError('Unknown PricingCurrency: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ProviderCatalogEntry`.
class ProviderCatalogEntry {
  const ProviderCatalogEntry({
    required this.providerTypeId,
    required this.displayName,
    required this.defaultEndpoint,
    required this.operations,
    required this.models,
  });

  factory ProviderCatalogEntry.fromJson(Map<String, Object?> json) {
    return ProviderCatalogEntry(
      providerTypeId: json['providerTypeId'] as String,
      displayName: json['displayName'] as String,
      defaultEndpoint: json['defaultEndpoint'] as String,
      operations: (json['operations'] as List<Object?>).map((item) => ProviderOperationSpec.fromJson(item as Map<String, Object?>)).toList(growable: false),
      models: (json['models'] as List<Object?>).map((item) => ModelCatalogEntry.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'providerTypeId': providerTypeId,
      'displayName': displayName,
      'defaultEndpoint': defaultEndpoint,
      'operations': operations.map((item) => item.toJson()).toList(growable: false),
      'models': models.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `providerTypeId` serialized as `providerTypeId`.
  final String providerTypeId;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `defaultEndpoint` serialized as `defaultEndpoint`.
  final String defaultEndpoint;
  /// Rust field `operations` serialized as `operations`.
  final List<ProviderOperationSpec> operations;
  /// Rust field `models` serialized as `models`.
  final List<ModelCatalogEntry> models;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ProviderModelSummary`.
class ProviderModelSummary {
  const ProviderModelSummary({
    required this.providerId,
    required this.providerName,
    required this.providerTypeId,
    required this.endpoint,
    required this.modelId,
    required this.capabilities,
    required this.pricing,
  });

  factory ProviderModelSummary.fromJson(Map<String, Object?> json) {
    return ProviderModelSummary(
      providerId: json['providerId'] as String,
      providerName: json['providerName'] as String,
      providerTypeId: json['providerTypeId'] as String,
      endpoint: json['endpoint'] as String,
      modelId: json['modelId'] as String,
      capabilities: ModelCapabilities.fromJson(json['capabilities'] as Map<String, Object?>),
      pricing: json['pricing'] == null ? null : ModelPricing.fromJson(json['pricing'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'providerId': providerId,
      'providerName': providerName,
      'providerTypeId': providerTypeId,
      'endpoint': endpoint,
      'modelId': modelId,
      'capabilities': capabilities.toJson(),
      'pricing': pricing?.toJson(),
    };
  }

  /// Rust field `providerId` serialized as `providerId`.
  final String providerId;
  /// Rust field `providerName` serialized as `providerName`.
  final String providerName;
  /// Rust field `providerTypeId` serialized as `providerTypeId`.
  final String providerTypeId;
  /// Rust field `endpoint` serialized as `endpoint`.
  final String endpoint;
  /// Rust field `modelId` serialized as `modelId`.
  final String modelId;
  /// Rust field `capabilities` serialized as `capabilities`.
  final ModelCapabilities capabilities;
  /// Rust field `pricing` serialized as `pricing`.
  final ModelPricing? pricing;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ProviderOperationResultSpec`.
class ProviderOperationResultSpec {
  const ProviderOperationResultSpec({
    required this.itemsJsonPath,
    required this.itemIdJsonPath,
    required this.inputPricePerTokenJsonPath,
    required this.cachedInputPricePerTokenJsonPath,
    required this.outputPricePerTokenJsonPath,
    required this.pricePerRequestJsonPath,
    required this.currencyJsonPath,
    required this.maxContextLengthJsonPath,
    required this.directImageJsonPath,
    required this.directAudioJsonPath,
    required this.directVideoJsonPath,
    required this.toolCallJsonPath,
    required this.supportsStructuredToolsJsonPath,
    required this.amountJsonPath,
    required this.amountCurrencyJsonPath,
  });

  factory ProviderOperationResultSpec.fromJson(Map<String, Object?> json) {
    return ProviderOperationResultSpec(
      itemsJsonPath: json['itemsJsonPath'] == null ? null : json['itemsJsonPath'] as String,
      itemIdJsonPath: json['itemIdJsonPath'] == null ? null : json['itemIdJsonPath'] as String,
      inputPricePerTokenJsonPath: json['inputPricePerTokenJsonPath'] == null ? null : json['inputPricePerTokenJsonPath'] as String,
      cachedInputPricePerTokenJsonPath: json['cachedInputPricePerTokenJsonPath'] == null ? null : json['cachedInputPricePerTokenJsonPath'] as String,
      outputPricePerTokenJsonPath: json['outputPricePerTokenJsonPath'] == null ? null : json['outputPricePerTokenJsonPath'] as String,
      pricePerRequestJsonPath: json['pricePerRequestJsonPath'] == null ? null : json['pricePerRequestJsonPath'] as String,
      currencyJsonPath: json['currencyJsonPath'] == null ? null : json['currencyJsonPath'] as String,
      maxContextLengthJsonPath: json['maxContextLengthJsonPath'] == null ? null : json['maxContextLengthJsonPath'] as String,
      directImageJsonPath: json['directImageJsonPath'] == null ? null : json['directImageJsonPath'] as String,
      directAudioJsonPath: json['directAudioJsonPath'] == null ? null : json['directAudioJsonPath'] as String,
      directVideoJsonPath: json['directVideoJsonPath'] == null ? null : json['directVideoJsonPath'] as String,
      toolCallJsonPath: json['toolCallJsonPath'] == null ? null : json['toolCallJsonPath'] as String,
      supportsStructuredToolsJsonPath: json['supportsStructuredToolsJsonPath'] == null ? null : json['supportsStructuredToolsJsonPath'] as String,
      amountJsonPath: json['amountJsonPath'] == null ? null : json['amountJsonPath'] as String,
      amountCurrencyJsonPath: json['amountCurrencyJsonPath'] == null ? null : json['amountCurrencyJsonPath'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'itemsJsonPath': itemsJsonPath,
      'itemIdJsonPath': itemIdJsonPath,
      'inputPricePerTokenJsonPath': inputPricePerTokenJsonPath,
      'cachedInputPricePerTokenJsonPath': cachedInputPricePerTokenJsonPath,
      'outputPricePerTokenJsonPath': outputPricePerTokenJsonPath,
      'pricePerRequestJsonPath': pricePerRequestJsonPath,
      'currencyJsonPath': currencyJsonPath,
      'maxContextLengthJsonPath': maxContextLengthJsonPath,
      'directImageJsonPath': directImageJsonPath,
      'directAudioJsonPath': directAudioJsonPath,
      'directVideoJsonPath': directVideoJsonPath,
      'toolCallJsonPath': toolCallJsonPath,
      'supportsStructuredToolsJsonPath': supportsStructuredToolsJsonPath,
      'amountJsonPath': amountJsonPath,
      'amountCurrencyJsonPath': amountCurrencyJsonPath,
    };
  }

  /// Rust field `itemsJsonPath` serialized as `itemsJsonPath`.
  final String? itemsJsonPath;
  /// Rust field `itemIdJsonPath` serialized as `itemIdJsonPath`.
  final String? itemIdJsonPath;
  /// Rust field `inputPricePerTokenJsonPath` serialized as `inputPricePerTokenJsonPath`.
  final String? inputPricePerTokenJsonPath;
  /// Rust field `cachedInputPricePerTokenJsonPath` serialized as `cachedInputPricePerTokenJsonPath`.
  final String? cachedInputPricePerTokenJsonPath;
  /// Rust field `outputPricePerTokenJsonPath` serialized as `outputPricePerTokenJsonPath`.
  final String? outputPricePerTokenJsonPath;
  /// Rust field `pricePerRequestJsonPath` serialized as `pricePerRequestJsonPath`.
  final String? pricePerRequestJsonPath;
  /// Rust field `currencyJsonPath` serialized as `currencyJsonPath`.
  final String? currencyJsonPath;
  /// Rust field `maxContextLengthJsonPath` serialized as `maxContextLengthJsonPath`.
  final String? maxContextLengthJsonPath;
  /// Rust field `directImageJsonPath` serialized as `directImageJsonPath`.
  final String? directImageJsonPath;
  /// Rust field `directAudioJsonPath` serialized as `directAudioJsonPath`.
  final String? directAudioJsonPath;
  /// Rust field `directVideoJsonPath` serialized as `directVideoJsonPath`.
  final String? directVideoJsonPath;
  /// Rust field `toolCallJsonPath` serialized as `toolCallJsonPath`.
  final String? toolCallJsonPath;
  /// Rust field `supportsStructuredToolsJsonPath` serialized as `supportsStructuredToolsJsonPath`.
  final String? supportsStructuredToolsJsonPath;
  /// Rust field `amountJsonPath` serialized as `amountJsonPath`.
  final String? amountJsonPath;
  /// Rust field `amountCurrencyJsonPath` serialized as `amountCurrencyJsonPath`.
  final String? amountCurrencyJsonPath;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ProviderOperationSpec`.
class ProviderOperationSpec {
  const ProviderOperationSpec({
    required this.operationType,
    required this.handlerId,
    required this.method,
    required this.path,
    required this.requiresApiKey,
    required this.result,
  });

  factory ProviderOperationSpec.fromJson(Map<String, Object?> json) {
    return ProviderOperationSpec(
      operationType: json['operationType'] as String,
      handlerId: json['handlerId'] as String,
      method: json['method'] as String,
      path: json['path'] as String,
      requiresApiKey: json['requiresApiKey'] as bool,
      result: ProviderOperationResultSpec.fromJson(json['result'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'operationType': operationType,
      'handlerId': handlerId,
      'method': method,
      'path': path,
      'requiresApiKey': requiresApiKey,
      'result': result.toJson(),
    };
  }

  /// Rust field `operationType` serialized as `operationType`.
  final String operationType;
  /// Rust field `handlerId` serialized as `handlerId`.
  final String handlerId;
  /// Rust field `method` serialized as `method`.
  final String method;
  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `requiresApiKey` serialized as `requiresApiKey`.
  final bool requiresApiKey;
  /// Rust field `result` serialized as `result`.
  final ProviderOperationResultSpec result;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ProviderProfile`.
class ProviderProfile {
  const ProviderProfile({
    required this.id,
    required this.name,
    required this.providerTypeId,
    required this.providerType,
    required this.endpoint,
    required this.apiKey,
    required this.useMultipleApiKeys,
    required this.apiKeyPool,
    required this.currentKeyIndex,
    required this.keyRotationMode,
    required this.customHeaders,
    required this.requestLimitPerMinute,
    required this.maxConcurrentRequests,
    required this.models,
  });

  factory ProviderProfile.fromJson(Map<String, Object?> json) {
    return ProviderProfile(
      id: json['id'] as String,
      name: json['name'] as String,
      providerTypeId: json['providerTypeId'] as String,
      providerType: ApiProviderType.fromJson(json['providerType']),
      endpoint: json['endpoint'] as String,
      apiKey: json['apiKey'] as String,
      useMultipleApiKeys: json['useMultipleApiKeys'] as bool,
      apiKeyPool: (json['apiKeyPool'] as List<Object?>).map((item) => item).toList(growable: false),
      currentKeyIndex: json['currentKeyIndex'] as int,
      keyRotationMode: json['keyRotationMode'] as String,
      customHeaders: json['customHeaders'] as String,
      requestLimitPerMinute: json['requestLimitPerMinute'] as int,
      maxConcurrentRequests: json['maxConcurrentRequests'] as int,
      models: (json['models'] as List<Object?>).map((item) => ModelProfile.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
      'providerTypeId': providerTypeId,
      'providerType': providerType.toJson(),
      'endpoint': endpoint,
      'apiKey': apiKey,
      'useMultipleApiKeys': useMultipleApiKeys,
      'apiKeyPool': apiKeyPool.map((item) => item).toList(growable: false),
      'currentKeyIndex': currentKeyIndex,
      'keyRotationMode': keyRotationMode,
      'customHeaders': customHeaders,
      'requestLimitPerMinute': requestLimitPerMinute,
      'maxConcurrentRequests': maxConcurrentRequests,
      'models': models.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `providerTypeId` serialized as `providerTypeId`.
  final String providerTypeId;
  /// Rust field `providerType` serialized as `providerType`.
  final ApiProviderType providerType;
  /// Rust field `endpoint` serialized as `endpoint`.
  final String endpoint;
  /// Rust field `apiKey` serialized as `apiKey`.
  final String apiKey;
  /// Rust field `useMultipleApiKeys` serialized as `useMultipleApiKeys`.
  final bool useMultipleApiKeys;
  /// Rust field `apiKeyPool` serialized as `apiKeyPool`.
  final List<Object?> apiKeyPool;
  /// Rust field `currentKeyIndex` serialized as `currentKeyIndex`.
  final int currentKeyIndex;
  /// Rust field `keyRotationMode` serialized as `keyRotationMode`.
  final String keyRotationMode;
  /// Rust field `customHeaders` serialized as `customHeaders`.
  final String customHeaders;
  /// Rust field `requestLimitPerMinute` serialized as `requestLimitPerMinute`.
  final int requestLimitPerMinute;
  /// Rust field `maxConcurrentRequests` serialized as `maxConcurrentRequests`.
  final int maxConcurrentRequests;
  /// Rust field `models` serialized as `models`.
  final List<ModelProfile> models;
}

/// Generated Dart model for Rust type `operit_model::ModelConfigData::ResolvedModelConfig`.
class ResolvedModelConfig {
  const ResolvedModelConfig({
    required this.providerId,
    required this.providerName,
    required this.modelId,
    required this.apiKey,
    required this.apiEndpoint,
    required this.apiProviderType,
    required this.apiProviderTypeId,
    required this.useMultipleApiKeys,
    required this.apiKeyPool,
    required this.currentKeyIndex,
    required this.keyRotationMode,
    required this.customHeaders,
    required this.requestLimitPerMinute,
    required this.maxConcurrentRequests,
    required this.pricing,
    required this.context,
    required this.capabilities,
    required this.builtinTools,
    required this.request,
    required this.parameters,
    required this.summary,
    required this.localRuntime,
  });

  factory ResolvedModelConfig.fromJson(Map<String, Object?> json) {
    return ResolvedModelConfig(
      providerId: json['providerId'] as String,
      providerName: json['providerName'] as String,
      modelId: json['modelId'] as String,
      apiKey: json['apiKey'] as String,
      apiEndpoint: json['apiEndpoint'] as String,
      apiProviderType: ApiProviderType.fromJson(json['apiProviderType']),
      apiProviderTypeId: json['apiProviderTypeId'] as String,
      useMultipleApiKeys: json['useMultipleApiKeys'] as bool,
      apiKeyPool: (json['apiKeyPool'] as List<Object?>).map((item) => item).toList(growable: false),
      currentKeyIndex: json['currentKeyIndex'] as int,
      keyRotationMode: json['keyRotationMode'] as String,
      customHeaders: json['customHeaders'] as String,
      requestLimitPerMinute: json['requestLimitPerMinute'] as int,
      maxConcurrentRequests: json['maxConcurrentRequests'] as int,
      pricing: json['pricing'] == null ? null : ModelPricing.fromJson(json['pricing'] as Map<String, Object?>),
      context: ModelContextSpec.fromJson(json['context'] as Map<String, Object?>),
      capabilities: ModelCapabilities.fromJson(json['capabilities'] as Map<String, Object?>),
      builtinTools: (json['builtinTools'] as List<Object?>).map((item) => ModelBuiltinTool.fromJson(item as Map<String, Object?>)).toList(growable: false),
      request: ModelRequestSpec.fromJson(json['request'] as Map<String, Object?>),
      parameters: (json['parameters'] as List<Object?>).map((item) => item).toList(growable: false),
      summary: ModelSummarySettings.fromJson(json['summary'] as Map<String, Object?>),
      localRuntime: LocalModelRuntimeSettings.fromJson(json['localRuntime'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'providerId': providerId,
      'providerName': providerName,
      'modelId': modelId,
      'apiKey': apiKey,
      'apiEndpoint': apiEndpoint,
      'apiProviderType': apiProviderType.toJson(),
      'apiProviderTypeId': apiProviderTypeId,
      'useMultipleApiKeys': useMultipleApiKeys,
      'apiKeyPool': apiKeyPool.map((item) => item).toList(growable: false),
      'currentKeyIndex': currentKeyIndex,
      'keyRotationMode': keyRotationMode,
      'customHeaders': customHeaders,
      'requestLimitPerMinute': requestLimitPerMinute,
      'maxConcurrentRequests': maxConcurrentRequests,
      'pricing': pricing?.toJson(),
      'context': context.toJson(),
      'capabilities': capabilities.toJson(),
      'builtinTools': builtinTools.map((item) => item.toJson()).toList(growable: false),
      'request': request.toJson(),
      'parameters': parameters.map((item) => item).toList(growable: false),
      'summary': summary.toJson(),
      'localRuntime': localRuntime.toJson(),
    };
  }

  /// Rust field `providerId` serialized as `providerId`.
  final String providerId;
  /// Rust field `providerName` serialized as `providerName`.
  final String providerName;
  /// Rust field `modelId` serialized as `modelId`.
  final String modelId;
  /// Rust field `apiKey` serialized as `apiKey`.
  final String apiKey;
  /// Rust field `apiEndpoint` serialized as `apiEndpoint`.
  final String apiEndpoint;
  /// Rust field `apiProviderType` serialized as `apiProviderType`.
  final ApiProviderType apiProviderType;
  /// Rust field `apiProviderTypeId` serialized as `apiProviderTypeId`.
  final String apiProviderTypeId;
  /// Rust field `useMultipleApiKeys` serialized as `useMultipleApiKeys`.
  final bool useMultipleApiKeys;
  /// Rust field `apiKeyPool` serialized as `apiKeyPool`.
  final List<Object?> apiKeyPool;
  /// Rust field `currentKeyIndex` serialized as `currentKeyIndex`.
  final int currentKeyIndex;
  /// Rust field `keyRotationMode` serialized as `keyRotationMode`.
  final String keyRotationMode;
  /// Rust field `customHeaders` serialized as `customHeaders`.
  final String customHeaders;
  /// Rust field `requestLimitPerMinute` serialized as `requestLimitPerMinute`.
  final int requestLimitPerMinute;
  /// Rust field `maxConcurrentRequests` serialized as `maxConcurrentRequests`.
  final int maxConcurrentRequests;
  /// Rust field `pricing` serialized as `pricing`.
  final ModelPricing? pricing;
  /// Rust field `context` serialized as `context`.
  final ModelContextSpec context;
  /// Rust field `capabilities` serialized as `capabilities`.
  final ModelCapabilities capabilities;
  /// Rust field `builtinTools` serialized as `builtinTools`.
  final List<ModelBuiltinTool> builtinTools;
  /// Rust field `request` serialized as `request`.
  final ModelRequestSpec request;
  /// Rust field `parameters` serialized as `parameters`.
  final List<Object?> parameters;
  /// Rust field `summary` serialized as `summary`.
  final ModelSummarySettings summary;
  /// Rust field `localRuntime` serialized as `localRuntime`.
  final LocalModelRuntimeSettings localRuntime;
}

/// Generated Dart model for Rust type `operit_model::OperitChatArchive::OperitArchivedChat`.
class OperitArchivedChat {
  const OperitArchivedChat({
    required this.id,
    required this.title,
    required this.messages,
    required this.createdAt,
    required this.updatedAt,
    required this.inputTokens,
    required this.outputTokens,
    required this.currentWindowSize,
    required this.group,
    required this.displayOrder,
    required this.workspace,
    required this.parentChatId,
    required this.characterCardName,
    required this.characterGroupId,
    required this.locked,
    required this.pinned,
  });

  factory OperitArchivedChat.fromJson(Map<String, Object?> json) {
    return OperitArchivedChat(
      id: json['id'] as String,
      title: json['title'] as String,
      messages: (json['messages'] as List<Object?>).map((item) => OperitArchivedMessage.fromJson(item as Map<String, Object?>)).toList(growable: false),
      createdAt: json['createdAt'] as String,
      updatedAt: json['updatedAt'] as String,
      inputTokens: json['inputTokens'] as int,
      outputTokens: json['outputTokens'] as int,
      currentWindowSize: json['currentWindowSize'] as int,
      group: json['group'] == null ? null : json['group'] as String,
      displayOrder: json['displayOrder'] as int,
      workspace: json['workspace'] == null ? null : json['workspace'] as String,
      parentChatId: json['parentChatId'] == null ? null : json['parentChatId'] as String,
      characterCardName: json['characterCardName'] == null ? null : json['characterCardName'] as String,
      characterGroupId: json['characterGroupId'] == null ? null : json['characterGroupId'] as String,
      locked: json['locked'] as bool,
      pinned: json['pinned'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'title': title,
      'messages': messages.map((item) => item.toJson()).toList(growable: false),
      'createdAt': createdAt,
      'updatedAt': updatedAt,
      'inputTokens': inputTokens,
      'outputTokens': outputTokens,
      'currentWindowSize': currentWindowSize,
      'group': group,
      'displayOrder': displayOrder,
      'workspace': workspace,
      'parentChatId': parentChatId,
      'characterCardName': characterCardName,
      'characterGroupId': characterGroupId,
      'locked': locked,
      'pinned': pinned,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `messages` serialized as `messages`.
  final List<OperitArchivedMessage> messages;
  /// Rust field `createdAt` serialized as `createdAt`.
  final String createdAt;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final String updatedAt;
  /// Rust field `inputTokens` serialized as `inputTokens`.
  final int inputTokens;
  /// Rust field `outputTokens` serialized as `outputTokens`.
  final int outputTokens;
  /// Rust field `currentWindowSize` serialized as `currentWindowSize`.
  final int currentWindowSize;
  /// Rust field `group` serialized as `group`.
  final String? group;
  /// Rust field `displayOrder` serialized as `displayOrder`.
  final int displayOrder;
  /// Rust field `workspace` serialized as `workspace`.
  final String? workspace;
  /// Rust field `parentChatId` serialized as `parentChatId`.
  final String? parentChatId;
  /// Rust field `characterCardName` serialized as `characterCardName`.
  final String? characterCardName;
  /// Rust field `characterGroupId` serialized as `characterGroupId`.
  final String? characterGroupId;
  /// Rust field `locked` serialized as `locked`.
  final bool locked;
  /// Rust field `pinned` serialized as `pinned`.
  final bool pinned;
}

/// Generated Dart model for Rust type `operit_model::OperitChatArchive::OperitArchivedMessage`.
class OperitArchivedMessage {
  const OperitArchivedMessage({
    required this.baseMessage,
    required this.variants,
  });

  factory OperitArchivedMessage.fromJson(Map<String, Object?> json) {
    return OperitArchivedMessage(
      baseMessage: json['baseMessage'],
      variants: (json['variants'] as List<Object?>).map((item) => OperitArchivedMessageVariant.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'baseMessage': baseMessage,
      'variants': variants.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `baseMessage` serialized as `baseMessage`.
  final Object? baseMessage;
  /// Rust field `variants` serialized as `variants`.
  final List<OperitArchivedMessageVariant> variants;
}

/// Generated Dart model for Rust type `operit_model::OperitChatArchive::OperitArchivedMessageVariant`.
class OperitArchivedMessageVariant {
  const OperitArchivedMessageVariant({
    required this.variantIndex,
    required this.parts,
    required this.roleName,
    required this.provider,
    required this.modelName,
    required this.inputTokens,
    required this.outputTokens,
    required this.cachedInputTokens,
    required this.sentAt,
    required this.outputDurationMs,
    required this.waitDurationMs,
    required this.completedAt,
  });

  factory OperitArchivedMessageVariant.fromJson(Map<String, Object?> json) {
    return OperitArchivedMessageVariant(
      variantIndex: json['variantIndex'] as int,
      parts: (json['parts'] as List<Object?>).map((item) => item).toList(growable: false),
      roleName: json['roleName'] as String,
      provider: json['provider'] as String,
      modelName: json['modelName'] as String,
      inputTokens: json['inputTokens'] as int,
      outputTokens: json['outputTokens'] as int,
      cachedInputTokens: json['cachedInputTokens'] as int,
      sentAt: json['sentAt'] as int,
      outputDurationMs: json['outputDurationMs'] as int,
      waitDurationMs: json['waitDurationMs'] as int,
      completedAt: json['completedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'variantIndex': variantIndex,
      'parts': parts.map((item) => item).toList(growable: false),
      'roleName': roleName,
      'provider': provider,
      'modelName': modelName,
      'inputTokens': inputTokens,
      'outputTokens': outputTokens,
      'cachedInputTokens': cachedInputTokens,
      'sentAt': sentAt,
      'outputDurationMs': outputDurationMs,
      'waitDurationMs': waitDurationMs,
      'completedAt': completedAt,
    };
  }

  /// Rust field `variantIndex` serialized as `variantIndex`.
  final int variantIndex;
  /// Rust field `parts` serialized as `parts`.
  final List<Object?> parts;
  /// Rust field `roleName` serialized as `roleName`.
  final String roleName;
  /// Rust field `provider` serialized as `provider`.
  final String provider;
  /// Rust field `modelName` serialized as `modelName`.
  final String modelName;
  /// Rust field `inputTokens` serialized as `inputTokens`.
  final int inputTokens;
  /// Rust field `outputTokens` serialized as `outputTokens`.
  final int outputTokens;
  /// Rust field `cachedInputTokens` serialized as `cachedInputTokens`.
  final int cachedInputTokens;
  /// Rust field `sentAt` serialized as `sentAt`.
  final int sentAt;
  /// Rust field `outputDurationMs` serialized as `outputDurationMs`.
  final int outputDurationMs;
  /// Rust field `waitDurationMs` serialized as `waitDurationMs`.
  final int waitDurationMs;
  /// Rust field `completedAt` serialized as `completedAt`.
  final int completedAt;
}

/// Generated Dart model for Rust type `operit_model::OperitChatArchive::OperitChatArchive`.
class OperitChatArchive {
  const OperitChatArchive({
    required this.archiveType,
    required this.formatVersion,
    required this.exportedAt,
    required this.chats,
  });

  factory OperitChatArchive.fromJson(Map<String, Object?> json) {
    return OperitChatArchive(
      archiveType: json['archiveType'] as String,
      formatVersion: json['formatVersion'] as int,
      exportedAt: json['exportedAt'] as int,
      chats: (json['chats'] as List<Object?>).map((item) => OperitArchivedChat.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'archiveType': archiveType,
      'formatVersion': formatVersion,
      'exportedAt': exportedAt,
      'chats': chats.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `archiveType` serialized as `archiveType`.
  final String archiveType;
  /// Rust field `formatVersion` serialized as `formatVersion`.
  final int formatVersion;
  /// Rust field `exportedAt` serialized as `exportedAt`.
  final int exportedAt;
  /// Rust field `chats` serialized as `chats`.
  final List<OperitArchivedChat> chats;
}

/// Generated Dart model for Rust type `operit_model::PendingQueueMessageItem::PendingQueueMessageItem`.
class PendingQueueMessageItem {
  const PendingQueueMessageItem({
    required this.id,
    required this.text,
  });

  factory PendingQueueMessageItem.fromJson(Map<String, Object?> json) {
    return PendingQueueMessageItem(
      id: json['id'] as int,
      text: json['text'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'text': text,
    };
  }

  /// Rust field `id` serialized as `id`.
  final int id;
  /// Rust field `text` serialized as `text`.
  final String text;
}

/// Generated Dart enum for Rust type `operit_model::PromptFunctionType::PromptFunctionType`.
enum PromptFunctionType {
  chat('CHAT'),
  voice('VOICE'),
  ;

  const PromptFunctionType(this.value);

  final String value;

  factory PromptFunctionType.fromJson(Object? value) {
    return switch (value) {
      'CHAT' => PromptFunctionType.chat,
      'VOICE' => PromptFunctionType.voice,
      _ => throw ArgumentError('Unknown PromptFunctionType: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_model::PromptTag::PromptTag`.
class PromptTag {
  const PromptTag({
    required this.id,
    required this.name,
    required this.description,
    required this.promptContent,
    required this.tagType,
    required this.createdAt,
    required this.updatedAt,
  });

  factory PromptTag.fromJson(Map<String, Object?> json) {
    return PromptTag(
      id: json['id'] as String,
      name: json['name'] as String,
      description: json['description'] as String,
      promptContent: json['promptContent'] as String,
      tagType: TagType.fromJson(json['tagType']),
      createdAt: json['createdAt'] as int,
      updatedAt: json['updatedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
      'description': description,
      'promptContent': promptContent,
      'tagType': tagType.toJson(),
      'createdAt': createdAt,
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `promptContent` serialized as `promptContent`.
  final String promptContent;
  /// Rust field `tagType` serialized as `tagType`.
  final TagType tagType;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
}

/// Generated Dart enum for Rust type `operit_model::PromptTag::TagType`.
enum TagType {
  tone('TONE'),
  character('CHARACTER'),
  function('FUNCTION'),
  custom('CUSTOM'),
  ;

  const TagType(this.value);

  final String value;

  factory TagType.fromJson(Object? value) {
    return switch (value) {
      'TONE' => TagType.tone,
      'CHARACTER' => TagType.character,
      'FUNCTION' => TagType.function,
      'CUSTOM' => TagType.custom,
      _ => throw ArgumentError('Unknown TagType: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_model::SttConfig::AvailableSttModel`.
class AvailableSttModel {
  const AvailableSttModel({
    required this.model,
    required this.displayName,
    required this.description,
    required this.languages,
  });

  factory AvailableSttModel.fromJson(Map<String, Object?> json) {
    return AvailableSttModel(
      model: json['model'] as String,
      displayName: json['displayName'] as String,
      description: json['description'] as String,
      languages: (json['languages'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'model': model,
      'displayName': displayName,
      'description': description,
      'languages': languages.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `model` serialized as `model`.
  final String model;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `languages` serialized as `languages`.
  final List<String> languages;
}

/// Generated Dart model for Rust type `operit_model::SttConfig::SttConfig`.
class SttConfig {
  const SttConfig({
    required this.id,
    required this.name,
    required this.providerType,
    required this.endpoint,
    required this.apiKey,
    required this.model,
    required this.fileFieldName,
    required this.modelFieldName,
    required this.languageFieldName,
    required this.responseTextJsonPath,
    required this.headers,
    required this.createdAt,
    required this.updatedAt,
  });

  factory SttConfig.fromJson(Map<String, Object?> json) {
    return SttConfig(
      id: json['id'] as String,
      name: json['name'] as String,
      providerType: json['providerType'] as String,
      endpoint: json['endpoint'] as String,
      apiKey: json['apiKey'] as String,
      model: json['model'] as String,
      fileFieldName: json['fileFieldName'] as String,
      modelFieldName: json['modelFieldName'] as String,
      languageFieldName: json['languageFieldName'] as String,
      responseTextJsonPath: json['responseTextJsonPath'] as String,
      headers: (json['headers'] as List<Object?>).map((item) => SttHttpHeader.fromJson(item as Map<String, Object?>)).toList(growable: false),
      createdAt: json['createdAt'] as int,
      updatedAt: json['updatedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
      'providerType': providerType,
      'endpoint': endpoint,
      'apiKey': apiKey,
      'model': model,
      'fileFieldName': fileFieldName,
      'modelFieldName': modelFieldName,
      'languageFieldName': languageFieldName,
      'responseTextJsonPath': responseTextJsonPath,
      'headers': headers.map((item) => item.toJson()).toList(growable: false),
      'createdAt': createdAt,
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `providerType` serialized as `providerType`.
  final String providerType;
  /// Rust field `endpoint` serialized as `endpoint`.
  final String endpoint;
  /// Rust field `apiKey` serialized as `apiKey`.
  final String apiKey;
  /// Rust field `model` serialized as `model`.
  final String model;
  /// Rust field `fileFieldName` serialized as `fileFieldName`.
  final String fileFieldName;
  /// Rust field `modelFieldName` serialized as `modelFieldName`.
  final String modelFieldName;
  /// Rust field `languageFieldName` serialized as `languageFieldName`.
  final String languageFieldName;
  /// Rust field `responseTextJsonPath` serialized as `responseTextJsonPath`.
  final String responseTextJsonPath;
  /// Rust field `headers` serialized as `headers`.
  final List<SttHttpHeader> headers;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
}

/// Generated Dart model for Rust type `operit_model::SttConfig::SttHttpHeader`.
class SttHttpHeader {
  const SttHttpHeader({
    required this.name,
    required this.value,
  });

  factory SttHttpHeader.fromJson(Map<String, Object?> json) {
    return SttHttpHeader(
      name: json['name'] as String,
      value: json['value'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'value': value,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `value` serialized as `value`.
  final String value;
}

/// Generated Dart model for Rust type `operit_model::SttConfig::SttProviderCatalogEntry`.
class SttProviderCatalogEntry {
  const SttProviderCatalogEntry({
    required this.providerTypeId,
    required this.displayName,
    required this.defaultEndpoint,
    required this.defaultModel,
    required this.defaultFileFieldName,
    required this.defaultModelFieldName,
    required this.defaultLanguageFieldName,
    required this.defaultResponseTextJsonPath,
    required this.defaultHeaders,
  });

  factory SttProviderCatalogEntry.fromJson(Map<String, Object?> json) {
    return SttProviderCatalogEntry(
      providerTypeId: json['providerTypeId'] as String,
      displayName: json['displayName'] as String,
      defaultEndpoint: json['defaultEndpoint'] as String,
      defaultModel: json['defaultModel'] as String,
      defaultFileFieldName: json['defaultFileFieldName'] as String,
      defaultModelFieldName: json['defaultModelFieldName'] as String,
      defaultLanguageFieldName: json['defaultLanguageFieldName'] as String,
      defaultResponseTextJsonPath: json['defaultResponseTextJsonPath'] as String,
      defaultHeaders: (json['defaultHeaders'] as List<Object?>).map((item) => SttHttpHeader.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'providerTypeId': providerTypeId,
      'displayName': displayName,
      'defaultEndpoint': defaultEndpoint,
      'defaultModel': defaultModel,
      'defaultFileFieldName': defaultFileFieldName,
      'defaultModelFieldName': defaultModelFieldName,
      'defaultLanguageFieldName': defaultLanguageFieldName,
      'defaultResponseTextJsonPath': defaultResponseTextJsonPath,
      'defaultHeaders': defaultHeaders.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `providerTypeId` serialized as `providerTypeId`.
  final String providerTypeId;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `defaultEndpoint` serialized as `defaultEndpoint`.
  final String defaultEndpoint;
  /// Rust field `defaultModel` serialized as `defaultModel`.
  final String defaultModel;
  /// Rust field `defaultFileFieldName` serialized as `defaultFileFieldName`.
  final String defaultFileFieldName;
  /// Rust field `defaultModelFieldName` serialized as `defaultModelFieldName`.
  final String defaultModelFieldName;
  /// Rust field `defaultLanguageFieldName` serialized as `defaultLanguageFieldName`.
  final String defaultLanguageFieldName;
  /// Rust field `defaultResponseTextJsonPath` serialized as `defaultResponseTextJsonPath`.
  final String defaultResponseTextJsonPath;
  /// Rust field `defaultHeaders` serialized as `defaultHeaders`.
  final List<SttHttpHeader> defaultHeaders;
}

/// Generated Dart model for Rust type `operit_model::SttConfig::SttRecognitionResult`.
class SttRecognitionResult {
  const SttRecognitionResult({
    required this.text,
  });

  factory SttRecognitionResult.fromJson(Map<String, Object?> json) {
    return SttRecognitionResult(
      text: json['text'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'text': text,
    };
  }

  /// Rust field `text` serialized as `text`.
  final String text;
}

/// Generated Dart model for Rust type `operit_model::TtsConfig::AvailableTtsVoice`.
class AvailableTtsVoice {
  const AvailableTtsVoice({
    required this.model,
    required this.voice,
    required this.displayName,
    required this.description,
    required this.responseFormat,
    required this.speed,
  });

  factory AvailableTtsVoice.fromJson(Map<String, Object?> json) {
    return AvailableTtsVoice(
      model: json['model'] as String,
      voice: json['voice'] as String,
      displayName: json['displayName'] as String,
      description: json['description'] as String,
      responseFormat: json['responseFormat'] as String,
      speed: (json['speed'] as num).toDouble(),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'model': model,
      'voice': voice,
      'displayName': displayName,
      'description': description,
      'responseFormat': responseFormat,
      'speed': speed,
    };
  }

  /// Rust field `model` serialized as `model`.
  final String model;
  /// Rust field `voice` serialized as `voice`.
  final String voice;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `responseFormat` serialized as `responseFormat`.
  final String responseFormat;
  /// Rust field `speed` serialized as `speed`.
  final double speed;
}

/// Generated Dart model for Rust type `operit_model::TtsConfig::TtsConfig`.
class TtsConfig {
  const TtsConfig({
    required this.id,
    required this.name,
    required this.providerType,
    required this.endpoint,
    required this.apiKey,
    required this.model,
    required this.voice,
    required this.responseFormat,
    required this.speed,
    required this.httpMethod,
    required this.requestBody,
    required this.contentType,
    required this.headers,
    required this.responsePipeline,
    required this.createdAt,
    required this.updatedAt,
  });

  factory TtsConfig.fromJson(Map<String, Object?> json) {
    return TtsConfig(
      id: json['id'] as String,
      name: json['name'] as String,
      providerType: json['providerType'] as String,
      endpoint: json['endpoint'] as String,
      apiKey: json['apiKey'] as String,
      model: json['model'] as String,
      voice: json['voice'] as String,
      responseFormat: json['responseFormat'] as String,
      speed: (json['speed'] as num).toDouble(),
      httpMethod: json['httpMethod'] as String,
      requestBody: json['requestBody'] as String,
      contentType: json['contentType'] as String,
      headers: (json['headers'] as List<Object?>).map((item) => TtsHttpHeader.fromJson(item as Map<String, Object?>)).toList(growable: false),
      responsePipeline: (json['responsePipeline'] as List<Object?>).map((item) => TtsHttpResponsePipelineStep.fromJson(item as Map<String, Object?>)).toList(growable: false),
      createdAt: json['createdAt'] as int,
      updatedAt: json['updatedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
      'providerType': providerType,
      'endpoint': endpoint,
      'apiKey': apiKey,
      'model': model,
      'voice': voice,
      'responseFormat': responseFormat,
      'speed': speed,
      'httpMethod': httpMethod,
      'requestBody': requestBody,
      'contentType': contentType,
      'headers': headers.map((item) => item.toJson()).toList(growable: false),
      'responsePipeline': responsePipeline.map((item) => item.toJson()).toList(growable: false),
      'createdAt': createdAt,
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `providerType` serialized as `providerType`.
  final String providerType;
  /// Rust field `endpoint` serialized as `endpoint`.
  final String endpoint;
  /// Rust field `apiKey` serialized as `apiKey`.
  final String apiKey;
  /// Rust field `model` serialized as `model`.
  final String model;
  /// Rust field `voice` serialized as `voice`.
  final String voice;
  /// Rust field `responseFormat` serialized as `responseFormat`.
  final String responseFormat;
  /// Rust field `speed` serialized as `speed`.
  final double speed;
  /// Rust field `httpMethod` serialized as `httpMethod`.
  final String httpMethod;
  /// Rust field `requestBody` serialized as `requestBody`.
  final String requestBody;
  /// Rust field `contentType` serialized as `contentType`.
  final String contentType;
  /// Rust field `headers` serialized as `headers`.
  final List<TtsHttpHeader> headers;
  /// Rust field `responsePipeline` serialized as `responsePipeline`.
  final List<TtsHttpResponsePipelineStep> responsePipeline;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
}

/// Generated Dart model for Rust type `operit_model::TtsConfig::TtsHostPlaybackResult`.
class TtsHostPlaybackResult {
  const TtsHostPlaybackResult({
    required this.path,
    required this.active,
    required this.paused,
    required this.details,
  });

  factory TtsHostPlaybackResult.fromJson(Map<String, Object?> json) {
    return TtsHostPlaybackResult(
      path: json['path'] as String,
      active: json['active'] as bool,
      paused: json['paused'] as bool,
      details: json['details'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'active': active,
      'paused': paused,
      'details': details,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `active` serialized as `active`.
  final bool active;
  /// Rust field `paused` serialized as `paused`.
  final bool paused;
  /// Rust field `details` serialized as `details`.
  final String details;
}

/// Generated Dart model for Rust type `operit_model::TtsConfig::TtsHttpHeader`.
class TtsHttpHeader {
  const TtsHttpHeader({
    required this.name,
    required this.value,
  });

  factory TtsHttpHeader.fromJson(Map<String, Object?> json) {
    return TtsHttpHeader(
      name: json['name'] as String,
      value: json['value'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'value': value,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `value` serialized as `value`.
  final String value;
}

/// Generated Dart model for Rust type `operit_model::TtsConfig::TtsHttpResponsePipelineStep`.
class TtsHttpResponsePipelineStep {
  const TtsHttpResponsePipelineStep({
    required this.stepType,
    required this.path,
    required this.headers,
  });

  factory TtsHttpResponsePipelineStep.fromJson(Map<String, Object?> json) {
    return TtsHttpResponsePipelineStep(
      stepType: json['stepType'] as String,
      path: json['path'] as String,
      headers: (json['headers'] as List<Object?>).map((item) => TtsHttpHeader.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'stepType': stepType,
      'path': path,
      'headers': headers.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `stepType` serialized as `stepType`.
  final String stepType;
  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `headers` serialized as `headers`.
  final List<TtsHttpHeader> headers;
}

/// Generated Dart model for Rust type `operit_model::TtsConfig::TtsPlaybackResult`.
class TtsPlaybackResult {
  const TtsPlaybackResult({
    required this.path,
    required this.started,
    required this.details,
  });

  factory TtsPlaybackResult.fromJson(Map<String, Object?> json) {
    return TtsPlaybackResult(
      path: json['path'] as String,
      started: json['started'] as bool,
      details: json['details'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'started': started,
      'details': details,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `started` serialized as `started`.
  final bool started;
  /// Rust field `details` serialized as `details`.
  final String details;
}

/// Generated Dart model for Rust type `operit_model::TtsConfig::TtsProviderCatalogEntry`.
class TtsProviderCatalogEntry {
  const TtsProviderCatalogEntry({
    required this.providerTypeId,
    required this.displayName,
    required this.defaultEndpoint,
    required this.defaultModel,
    required this.defaultResponseFormat,
    required this.defaultHttpMethod,
    required this.defaultContentType,
    required this.defaultRequestBody,
    required this.defaultHeaders,
    required this.defaultResponsePipeline,
    required this.operations,
  });

  factory TtsProviderCatalogEntry.fromJson(Map<String, Object?> json) {
    return TtsProviderCatalogEntry(
      providerTypeId: json['providerTypeId'] as String,
      displayName: json['displayName'] as String,
      defaultEndpoint: json['defaultEndpoint'] as String,
      defaultModel: json['defaultModel'] as String,
      defaultResponseFormat: json['defaultResponseFormat'] as String,
      defaultHttpMethod: json['defaultHttpMethod'] as String,
      defaultContentType: json['defaultContentType'] as String,
      defaultRequestBody: json['defaultRequestBody'] as String,
      defaultHeaders: (json['defaultHeaders'] as List<Object?>).map((item) => TtsHttpHeader.fromJson(item as Map<String, Object?>)).toList(growable: false),
      defaultResponsePipeline: (json['defaultResponsePipeline'] as List<Object?>).map((item) => TtsHttpResponsePipelineStep.fromJson(item as Map<String, Object?>)).toList(growable: false),
      operations: (json['operations'] as List<Object?>).map((item) => TtsProviderOperationSpec.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'providerTypeId': providerTypeId,
      'displayName': displayName,
      'defaultEndpoint': defaultEndpoint,
      'defaultModel': defaultModel,
      'defaultResponseFormat': defaultResponseFormat,
      'defaultHttpMethod': defaultHttpMethod,
      'defaultContentType': defaultContentType,
      'defaultRequestBody': defaultRequestBody,
      'defaultHeaders': defaultHeaders.map((item) => item.toJson()).toList(growable: false),
      'defaultResponsePipeline': defaultResponsePipeline.map((item) => item.toJson()).toList(growable: false),
      'operations': operations.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `providerTypeId` serialized as `providerTypeId`.
  final String providerTypeId;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `defaultEndpoint` serialized as `defaultEndpoint`.
  final String defaultEndpoint;
  /// Rust field `defaultModel` serialized as `defaultModel`.
  final String defaultModel;
  /// Rust field `defaultResponseFormat` serialized as `defaultResponseFormat`.
  final String defaultResponseFormat;
  /// Rust field `defaultHttpMethod` serialized as `defaultHttpMethod`.
  final String defaultHttpMethod;
  /// Rust field `defaultContentType` serialized as `defaultContentType`.
  final String defaultContentType;
  /// Rust field `defaultRequestBody` serialized as `defaultRequestBody`.
  final String defaultRequestBody;
  /// Rust field `defaultHeaders` serialized as `defaultHeaders`.
  final List<TtsHttpHeader> defaultHeaders;
  /// Rust field `defaultResponsePipeline` serialized as `defaultResponsePipeline`.
  final List<TtsHttpResponsePipelineStep> defaultResponsePipeline;
  /// Rust field `operations` serialized as `operations`.
  final List<TtsProviderOperationSpec> operations;
}

/// Generated Dart model for Rust type `operit_model::TtsConfig::TtsProviderOperationResultSpec`.
class TtsProviderOperationResultSpec {
  const TtsProviderOperationResultSpec({
    required this.itemsJsonPath,
    required this.modelJsonPath,
    required this.voiceJsonPath,
    required this.displayNameJsonPath,
    required this.descriptionJsonPath,
  });

  factory TtsProviderOperationResultSpec.fromJson(Map<String, Object?> json) {
    return TtsProviderOperationResultSpec(
      itemsJsonPath: json['itemsJsonPath'] == null ? null : json['itemsJsonPath'] as String,
      modelJsonPath: json['modelJsonPath'] == null ? null : json['modelJsonPath'] as String,
      voiceJsonPath: json['voiceJsonPath'] == null ? null : json['voiceJsonPath'] as String,
      displayNameJsonPath: json['displayNameJsonPath'] == null ? null : json['displayNameJsonPath'] as String,
      descriptionJsonPath: json['descriptionJsonPath'] == null ? null : json['descriptionJsonPath'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'itemsJsonPath': itemsJsonPath,
      'modelJsonPath': modelJsonPath,
      'voiceJsonPath': voiceJsonPath,
      'displayNameJsonPath': displayNameJsonPath,
      'descriptionJsonPath': descriptionJsonPath,
    };
  }

  /// Rust field `itemsJsonPath` serialized as `itemsJsonPath`.
  final String? itemsJsonPath;
  /// Rust field `modelJsonPath` serialized as `modelJsonPath`.
  final String? modelJsonPath;
  /// Rust field `voiceJsonPath` serialized as `voiceJsonPath`.
  final String? voiceJsonPath;
  /// Rust field `displayNameJsonPath` serialized as `displayNameJsonPath`.
  final String? displayNameJsonPath;
  /// Rust field `descriptionJsonPath` serialized as `descriptionJsonPath`.
  final String? descriptionJsonPath;
}

/// Generated Dart model for Rust type `operit_model::TtsConfig::TtsProviderOperationSpec`.
class TtsProviderOperationSpec {
  const TtsProviderOperationSpec({
    required this.operationType,
    required this.handlerId,
    required this.method,
    required this.path,
    required this.requiresApiKey,
    required this.authHeaderName,
    required this.authHeaderValue,
    required this.body,
    required this.result,
  });

  factory TtsProviderOperationSpec.fromJson(Map<String, Object?> json) {
    return TtsProviderOperationSpec(
      operationType: json['operationType'] as String,
      handlerId: json['handlerId'] as String,
      method: json['method'] as String,
      path: json['path'] as String,
      requiresApiKey: json['requiresApiKey'] as bool,
      authHeaderName: json['authHeaderName'] as String,
      authHeaderValue: json['authHeaderValue'] as String,
      body: json['body'] as String,
      result: TtsProviderOperationResultSpec.fromJson(json['result'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'operationType': operationType,
      'handlerId': handlerId,
      'method': method,
      'path': path,
      'requiresApiKey': requiresApiKey,
      'authHeaderName': authHeaderName,
      'authHeaderValue': authHeaderValue,
      'body': body,
      'result': result.toJson(),
    };
  }

  /// Rust field `operationType` serialized as `operationType`.
  final String operationType;
  /// Rust field `handlerId` serialized as `handlerId`.
  final String handlerId;
  /// Rust field `method` serialized as `method`.
  final String method;
  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `requiresApiKey` serialized as `requiresApiKey`.
  final bool requiresApiKey;
  /// Rust field `authHeaderName` serialized as `authHeaderName`.
  final String authHeaderName;
  /// Rust field `authHeaderValue` serialized as `authHeaderValue`.
  final String authHeaderValue;
  /// Rust field `body` serialized as `body`.
  final String body;
  /// Rust field `result` serialized as `result`.
  final TtsProviderOperationResultSpec result;
}

/// Generated Dart model for Rust type `operit_model::TtsConfig::TtsSynthesisResult`.
class TtsSynthesisResult {
  const TtsSynthesisResult({
    required this.audioPaths,
    required this.audioStoragePaths,
  });

  factory TtsSynthesisResult.fromJson(Map<String, Object?> json) {
    return TtsSynthesisResult(
      audioPaths: (json['audioPaths'] as List<Object?>).map((item) => item as String).toList(growable: false),
      audioStoragePaths: (json['audioStoragePaths'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'audioPaths': audioPaths.map((item) => item).toList(growable: false),
      'audioStoragePaths': audioStoragePaths.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `audioPaths` serialized as `audioPaths`.
  final List<String> audioPaths;
  /// Rust field `audioStoragePaths` serialized as `audioStoragePaths`.
  final List<String> audioStoragePaths;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::javascript::JsToolPkgWasmArg`.
class JsToolPkgWasmArg {
  const JsToolPkgWasmArg({
    required this.valueType,
    required this.value,
  });

  factory JsToolPkgWasmArg.fromJson(Map<String, Object?> json) {
    return JsToolPkgWasmArg(
      valueType: json['type'] as String,
      value: json['value'],
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'type': valueType,
      'value': value,
    };
  }

  /// Rust field `value_type` serialized as `type`.
  final String valueType;
  /// Rust field `value` serialized as `value`.
  final Object? value;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::javascript::JsToolPkgWasmRequest`.
class JsToolPkgWasmRequest {
  const JsToolPkgWasmRequest({
    required this.packageTarget,
    required this.moduleId,
    required this.exportName,
    required this.args,
  });

  factory JsToolPkgWasmRequest.fromJson(Map<String, Object?> json) {
    return JsToolPkgWasmRequest(
      packageTarget: json['package_target'] as String,
      moduleId: json['module_id'] as String,
      exportName: json['export_name'] as String,
      args: (json['args'] as List<Object?>).map((item) => JsToolPkgWasmArg.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'package_target': packageTarget,
      'module_id': moduleId,
      'export_name': exportName,
      'args': args.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `package_target` serialized as `package_target`.
  final String packageTarget;
  /// Rust field `module_id` serialized as `module_id`.
  final String moduleId;
  /// Rust field `export_name` serialized as `export_name`.
  final String exportName;
  /// Rust field `args` serialized as `args`.
  final List<JsToolPkgWasmArg> args;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::javascript::JsToolPkgWasmResult`.
class JsToolPkgWasmResult {
  const JsToolPkgWasmResult({
    required this.valueType,
    required this.value,
  });

  factory JsToolPkgWasmResult.fromJson(Map<String, Object?> json) {
    return JsToolPkgWasmResult(
      valueType: json['value_type'] == null ? null : json['value_type'] as String,
      value: json['value'],
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'value_type': valueType,
      'value': value,
    };
  }

  /// Rust field `value_type` serialized as `value_type`.
  final String? valueType;
  /// Rust field `value` serialized as `value`.
  final Object? value;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::AgentStatusResultData`.
class AgentStatusResultData {
  const AgentStatusResultData({
    required this.chatId,
    required this.state,
    required this.message,
    required this.isIdle,
    required this.isProcessing,
  });

  factory AgentStatusResultData.fromJson(Map<String, Object?> json) {
    return AgentStatusResultData(
      chatId: json['chatId'] as String,
      state: json['state'] as String,
      message: json['message'],
      isIdle: json['isIdle'] as bool,
      isProcessing: json['isProcessing'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'chatId': chatId,
      'state': state,
      'message': message,
      'isIdle': isIdle,
      'isProcessing': isProcessing,
    };
  }

  /// Rust field `chatId` serialized as `chatId`.
  final String chatId;
  /// Rust field `state` serialized as `state`.
  final String state;
  /// Rust field `message` serialized as `message`.
  final Object? message;
  /// Rust field `isIdle` serialized as `isIdle`.
  final bool isIdle;
  /// Rust field `isProcessing` serialized as `isProcessing`.
  final bool isProcessing;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::AppListData`.
class AppListData {
  const AppListData({
    required this.includesSystemApps,
    required this.packages,
  });

  factory AppListData.fromJson(Map<String, Object?> json) {
    return AppListData(
      includesSystemApps: json['includesSystemApps'] as bool,
      packages: (json['packages'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'includesSystemApps': includesSystemApps,
      'packages': packages.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `includesSystemApps` serialized as `includesSystemApps`.
  final bool includesSystemApps;
  /// Rust field `packages` serialized as `packages`.
  final List<String> packages;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::AppOperationData`.
class AppOperationData {
  const AppOperationData({
    required this.operationType,
    required this.packageName,
    required this.success,
    required this.details,
  });

  factory AppOperationData.fromJson(Map<String, Object?> json) {
    return AppOperationData(
      operationType: json['operationType'] as String,
      packageName: json['packageName'] as String,
      success: json['success'] as bool,
      details: json['details'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'operationType': operationType,
      'packageName': packageName,
      'success': success,
      'details': details,
    };
  }

  /// Rust field `operationType` serialized as `operationType`.
  final String operationType;
  /// Rust field `packageName` serialized as `packageName`.
  final String packageName;
  /// Rust field `success` serialized as `success`.
  final bool success;
  /// Rust field `details` serialized as `details`.
  final String details;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::AppUsageTimeEntry`.
class AppUsageTimeEntry {
  const AppUsageTimeEntry({
    required this.packageName,
    required this.appName,
    required this.totalForegroundTimeMs,
    required this.lastTimeUsed,
    required this.isSystemApp,
  });

  factory AppUsageTimeEntry.fromJson(Map<String, Object?> json) {
    return AppUsageTimeEntry(
      packageName: json['packageName'] as String,
      appName: json['appName'] as String,
      totalForegroundTimeMs: json['totalForegroundTimeMs'] as int,
      lastTimeUsed: json['lastTimeUsed'] as int,
      isSystemApp: json['isSystemApp'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'packageName': packageName,
      'appName': appName,
      'totalForegroundTimeMs': totalForegroundTimeMs,
      'lastTimeUsed': lastTimeUsed,
      'isSystemApp': isSystemApp,
    };
  }

  /// Rust field `packageName` serialized as `packageName`.
  final String packageName;
  /// Rust field `appName` serialized as `appName`.
  final String appName;
  /// Rust field `totalForegroundTimeMs` serialized as `totalForegroundTimeMs`.
  final int totalForegroundTimeMs;
  /// Rust field `lastTimeUsed` serialized as `lastTimeUsed`.
  final int lastTimeUsed;
  /// Rust field `isSystemApp` serialized as `isSystemApp`.
  final bool isSystemApp;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::AppUsageTimeResultData`.
class AppUsageTimeResultData {
  const AppUsageTimeResultData({
    required this.startTime,
    required this.endTime,
    required this.sinceHours,
    required this.requestedPackageName,
    required this.includesSystemApps,
    required this.totalEntries,
    required this.entries,
  });

  factory AppUsageTimeResultData.fromJson(Map<String, Object?> json) {
    return AppUsageTimeResultData(
      startTime: json['startTime'] as int,
      endTime: json['endTime'] as int,
      sinceHours: json['sinceHours'] as int,
      requestedPackageName: json['requestedPackageName'] == null ? null : json['requestedPackageName'] as String,
      includesSystemApps: json['includesSystemApps'] as bool,
      totalEntries: json['totalEntries'] as int,
      entries: (json['entries'] as List<Object?>).map((item) => AppUsageTimeEntry.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'startTime': startTime,
      'endTime': endTime,
      'sinceHours': sinceHours,
      'requestedPackageName': requestedPackageName,
      'includesSystemApps': includesSystemApps,
      'totalEntries': totalEntries,
      'entries': entries.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `startTime` serialized as `startTime`.
  final int startTime;
  /// Rust field `endTime` serialized as `endTime`.
  final int endTime;
  /// Rust field `sinceHours` serialized as `sinceHours`.
  final int sinceHours;
  /// Rust field `requestedPackageName` serialized as `requestedPackageName`.
  final String? requestedPackageName;
  /// Rust field `includesSystemApps` serialized as `includesSystemApps`.
  final bool includesSystemApps;
  /// Rust field `totalEntries` serialized as `totalEntries`.
  final int totalEntries;
  /// Rust field `entries` serialized as `entries`.
  final List<AppUsageTimeEntry> entries;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BinaryFileContentData`.
class BinaryFileContentData {
  const BinaryFileContentData({
    required this.path,
    required this.contentBase64,
    required this.size,
  });

  factory BinaryFileContentData.fromJson(Map<String, Object?> json) {
    return BinaryFileContentData(
      path: json['path'] as String,
      contentBase64: json['contentBase64'] as String,
      size: json['size'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'contentBase64': contentBase64,
      'size': size,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `contentBase64` serialized as `contentBase64`.
  final String contentBase64;
  /// Rust field `size` serialized as `size`.
  final int size;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BinaryResultData`.
class BinaryResultData {
  const BinaryResultData({
    required this.value,
  });

  factory BinaryResultData.fromJson(Map<String, Object?> json) {
    return BinaryResultData(
      value: json['value'] as Uint8List,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'value': value,
    };
  }

  /// Rust field `value` serialized as `value`.
  final Uint8List value;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothBleCharacteristicData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothBleCharacteristicData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothBleCharacteristicData({
    required this.uuid,
    required this.properties,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothBleCharacteristicData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothBleCharacteristicData(
      uuid: json['uuid'] as String,
      properties: (json['properties'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'uuid': uuid,
      'properties': properties.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `uuid` serialized as `uuid`.
  final String uuid;
  /// Rust field `properties` serialized as `properties`.
  final List<String> properties;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothBleNotificationData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationData({
    required this.sessionId,
    required this.notifications,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationData(
      sessionId: json['sessionId'] as String,
      notifications: (json['notifications'] as List<Object?>).map((item) => CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationEntry.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'notifications': notifications.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `notifications` serialized as `notifications`.
  final List<CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationEntry> notifications;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothBleNotificationEntry`.
class CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationEntry {
  const CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationEntry({
    required this.characteristicUuid,
    required this.bytesRead,
    required this.text,
    required this.dataBase64,
    required this.timestamp,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationEntry.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationEntry(
      characteristicUuid: json['characteristicUuid'] as String,
      bytesRead: json['bytesRead'] as int,
      text: json['text'] == null ? null : json['text'] as String,
      dataBase64: json['dataBase64'] == null ? null : json['dataBase64'] as String,
      timestamp: json['timestamp'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'characteristicUuid': characteristicUuid,
      'bytesRead': bytesRead,
      'text': text,
      'dataBase64': dataBase64,
      'timestamp': timestamp,
    };
  }

  /// Rust field `characteristicUuid` serialized as `characteristicUuid`.
  final String characteristicUuid;
  /// Rust field `bytesRead` serialized as `bytesRead`.
  final int bytesRead;
  /// Rust field `text` serialized as `text`.
  final String? text;
  /// Rust field `dataBase64` serialized as `dataBase64`.
  final String? dataBase64;
  /// Rust field `timestamp` serialized as `timestamp`.
  final int timestamp;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothBleServiceData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothBleServiceData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothBleServiceData({
    required this.uuid,
    required this.characteristics,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothBleServiceData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothBleServiceData(
      uuid: json['uuid'] as String,
      characteristics: (json['characteristics'] as List<Object?>).map((item) => CoreOperitPluginSdkJsSdkResultsBluetoothBleCharacteristicData.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'uuid': uuid,
      'characteristics': characteristics.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `uuid` serialized as `uuid`.
  final String uuid;
  /// Rust field `characteristics` serialized as `characteristics`.
  final List<CoreOperitPluginSdkJsSdkResultsBluetoothBleCharacteristicData> characteristics;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothBleServicesData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothBleServicesData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothBleServicesData({
    required this.sessionId,
    required this.services,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothBleServicesData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothBleServicesData(
      sessionId: json['sessionId'] as String,
      services: (json['services'] as List<Object?>).map((item) => CoreOperitPluginSdkJsSdkResultsBluetoothBleServiceData.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'services': services.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `services` serialized as `services`.
  final List<CoreOperitPluginSdkJsSdkResultsBluetoothBleServiceData> services;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothBondedDevicesData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothBondedDevicesData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothBondedDevicesData({
    required this.devices,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothBondedDevicesData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothBondedDevicesData(
      devices: (json['devices'] as List<Object?>).map((item) => CoreOperitPluginSdkJsSdkResultsBluetoothDeviceData.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'devices': devices.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `devices` serialized as `devices`.
  final List<CoreOperitPluginSdkJsSdkResultsBluetoothDeviceData> devices;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothDeviceData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothDeviceData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothDeviceData({
    required this.name,
    required this.address,
    required this.type,
    required this.bondState,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothDeviceData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothDeviceData(
      name: json['name'] == null ? null : json['name'] as String,
      address: json['address'] as String,
      type: json['type'] as String,
      bondState: json['bondState'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'address': address,
      'type': type,
      'bondState': bondState,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String? name;
  /// Rust field `address` serialized as `address`.
  final String address;
  /// Rust field `r#type` serialized as `type`.
  final String type;
  /// Rust field `bondState` serialized as `bondState`.
  final String bondState;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothReadData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothReadData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothReadData({
    required this.sessionId,
    required this.bytesRead,
    required this.text,
    required this.dataBase64,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothReadData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothReadData(
      sessionId: json['sessionId'] as String,
      bytesRead: json['bytesRead'] as int,
      text: json['text'] == null ? null : json['text'] as String,
      dataBase64: json['dataBase64'] == null ? null : json['dataBase64'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'bytesRead': bytesRead,
      'text': text,
      'dataBase64': dataBase64,
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `bytesRead` serialized as `bytesRead`.
  final int bytesRead;
  /// Rust field `text` serialized as `text`.
  final String? text;
  /// Rust field `dataBase64` serialized as `dataBase64`.
  final String? dataBase64;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothScanResultData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothScanResultData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothScanResultData({
    required this.devices,
    required this.durationMs,
    required this.includesBle,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothScanResultData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothScanResultData(
      devices: (json['devices'] as List<Object?>).map((item) => CoreOperitPluginSdkJsSdkResultsBluetoothScannedDeviceData.fromJson(item as Map<String, Object?>)).toList(growable: false),
      durationMs: json['durationMs'] as int,
      includesBle: json['includesBle'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'devices': devices.map((item) => item.toJson()).toList(growable: false),
      'durationMs': durationMs,
      'includesBle': includesBle,
    };
  }

  /// Rust field `devices` serialized as `devices`.
  final List<CoreOperitPluginSdkJsSdkResultsBluetoothScannedDeviceData> devices;
  /// Rust field `durationMs` serialized as `durationMs`.
  final int durationMs;
  /// Rust field `includesBle` serialized as `includesBle`.
  final bool includesBle;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothScannedDeviceData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothScannedDeviceData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothScannedDeviceData({
    required this.name,
    required this.address,
    required this.type,
    required this.bondState,
    required this.source,
    required this.rssi,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothScannedDeviceData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothScannedDeviceData(
      name: json['name'] == null ? null : json['name'] as String,
      address: json['address'] as String,
      type: json['type'] as String,
      bondState: json['bondState'] as String,
      source: json['source'] as String,
      rssi: json['rssi'] == null ? null : json['rssi'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'address': address,
      'type': type,
      'bondState': bondState,
      'source': source,
      'rssi': rssi,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String? name;
  /// Rust field `address` serialized as `address`.
  final String address;
  /// Rust field `r#type` serialized as `type`.
  final String type;
  /// Rust field `bondState` serialized as `bondState`.
  final String bondState;
  /// Rust field `source` serialized as `source`.
  final String source;
  /// Rust field `rssi` serialized as `rssi`.
  final int? rssi;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothSessionData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothSessionData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothSessionData({
    required this.sessionId,
    required this.address,
    required this.mode,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothSessionData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothSessionData(
      sessionId: json['sessionId'] as String,
      address: json['address'] as String,
      mode: json['mode'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'address': address,
      'mode': mode,
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `address` serialized as `address`.
  final String address;
  /// Rust field `mode` serialized as `mode`.
  final String mode;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothStateData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothStateData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothStateData({
    required this.supported,
    required this.enabled,
    required this.state,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothStateData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothStateData(
      supported: json['supported'] as bool,
      enabled: json['enabled'] as bool,
      state: json['state'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'supported': supported,
      'enabled': enabled,
      'state': state,
    };
  }

  /// Rust field `supported` serialized as `supported`.
  final bool supported;
  /// Rust field `enabled` serialized as `enabled`.
  final bool enabled;
  /// Rust field `state` serialized as `state`.
  final String state;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BluetoothTransferData`.
class CoreOperitPluginSdkJsSdkResultsBluetoothTransferData {
  const CoreOperitPluginSdkJsSdkResultsBluetoothTransferData({
    required this.sessionId,
    required this.bytesWritten,
  });

  factory CoreOperitPluginSdkJsSdkResultsBluetoothTransferData.fromJson(Map<String, Object?> json) {
    return CoreOperitPluginSdkJsSdkResultsBluetoothTransferData(
      sessionId: json['sessionId'] as String,
      bytesWritten: json['bytesWritten'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'bytesWritten': bytesWritten,
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `bytesWritten` serialized as `bytesWritten`.
  final int bytesWritten;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::BooleanResultData`.
class BooleanResultData {
  const BooleanResultData({
    required this.value,
  });

  factory BooleanResultData.fromJson(Map<String, Object?> json) {
    return BooleanResultData(
      value: json['value'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'value': value,
    };
  }

  /// Rust field `value` serialized as `value`.
  final bool value;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::CharacterCardInfo`.
class CharacterCardInfo {
  const CharacterCardInfo({
    required this.id,
    required this.name,
    required this.description,
    required this.isDefault,
    required this.createdAt,
    required this.updatedAt,
  });

  factory CharacterCardInfo.fromJson(Map<String, Object?> json) {
    return CharacterCardInfo(
      id: json['id'] as String,
      name: json['name'] as String,
      description: json['description'] as String,
      isDefault: json['isDefault'] as bool,
      createdAt: json['createdAt'] as int,
      updatedAt: json['updatedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
      'description': description,
      'isDefault': isDefault,
      'createdAt': createdAt,
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `isDefault` serialized as `isDefault`.
  final bool isDefault;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::CharacterCardListResultData`.
class CharacterCardListResultData {
  const CharacterCardListResultData({
    required this.totalCount,
    required this.cards,
  });

  factory CharacterCardListResultData.fromJson(Map<String, Object?> json) {
    return CharacterCardListResultData(
      totalCount: json['totalCount'] as int,
      cards: (json['cards'] as List<Object?>).map((item) => CharacterCardInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'totalCount': totalCount,
      'cards': cards.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `totalCount` serialized as `totalCount`.
  final int totalCount;
  /// Rust field `cards` serialized as `cards`.
  final List<CharacterCardInfo> cards;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::ChatCreationResultData`.
class ChatCreationResultData {
  const ChatCreationResultData({
    required this.chatId,
    required this.createdAt,
  });

  factory ChatCreationResultData.fromJson(Map<String, Object?> json) {
    return ChatCreationResultData(
      chatId: json['chatId'] as String,
      createdAt: json['createdAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'chatId': chatId,
      'createdAt': createdAt,
    };
  }

  /// Rust field `chatId` serialized as `chatId`.
  final String chatId;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::ChatDeleteResultData`.
class ChatDeleteResultData {
  const ChatDeleteResultData({
    required this.chatId,
    required this.deletedAt,
  });

  factory ChatDeleteResultData.fromJson(Map<String, Object?> json) {
    return ChatDeleteResultData(
      chatId: json['chatId'] as String,
      deletedAt: json['deletedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'chatId': chatId,
      'deletedAt': deletedAt,
    };
  }

  /// Rust field `chatId` serialized as `chatId`.
  final String chatId;
  /// Rust field `deletedAt` serialized as `deletedAt`.
  final int deletedAt;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::ChatFindResultData`.
class ChatFindResultData {
  const ChatFindResultData({
    required this.matchedCount,
    required this.chat,
  });

  factory ChatFindResultData.fromJson(Map<String, Object?> json) {
    return ChatFindResultData(
      matchedCount: json['matchedCount'] as int,
      chat: json['chat'],
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'matchedCount': matchedCount,
      'chat': chat,
    };
  }

  /// Rust field `matchedCount` serialized as `matchedCount`.
  final int matchedCount;
  /// Rust field `chat` serialized as `chat`.
  final Object? chat;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::ChatInfo`.
class ChatInfo {
  const ChatInfo({
    required this.id,
    required this.title,
    required this.messageCount,
    required this.createdAt,
    required this.updatedAt,
    required this.isCurrent,
    required this.inputTokens,
    required this.outputTokens,
    required this.characterCardName,
  });

  factory ChatInfo.fromJson(Map<String, Object?> json) {
    return ChatInfo(
      id: json['id'] as String,
      title: json['title'] as String,
      messageCount: json['messageCount'] as int,
      createdAt: json['createdAt'] as String,
      updatedAt: json['updatedAt'] as String,
      isCurrent: json['isCurrent'] as bool,
      inputTokens: json['inputTokens'] as int,
      outputTokens: json['outputTokens'] as int,
      characterCardName: json['characterCardName'],
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'title': title,
      'messageCount': messageCount,
      'createdAt': createdAt,
      'updatedAt': updatedAt,
      'isCurrent': isCurrent,
      'inputTokens': inputTokens,
      'outputTokens': outputTokens,
      'characterCardName': characterCardName,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `messageCount` serialized as `messageCount`.
  final int messageCount;
  /// Rust field `createdAt` serialized as `createdAt`.
  final String createdAt;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final String updatedAt;
  /// Rust field `isCurrent` serialized as `isCurrent`.
  final bool isCurrent;
  /// Rust field `inputTokens` serialized as `inputTokens`.
  final int inputTokens;
  /// Rust field `outputTokens` serialized as `outputTokens`.
  final int outputTokens;
  /// Rust field `characterCardName` serialized as `characterCardName`.
  final Object? characterCardName;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::ChatListResultData`.
class ChatListResultData {
  const ChatListResultData({
    required this.totalCount,
    required this.currentChatId,
    required this.chats,
  });

  factory ChatListResultData.fromJson(Map<String, Object?> json) {
    return ChatListResultData(
      totalCount: json['totalCount'] as int,
      currentChatId: json['currentChatId'],
      chats: (json['chats'] as List<Object?>).map((item) => ChatInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'totalCount': totalCount,
      'currentChatId': currentChatId,
      'chats': chats.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `totalCount` serialized as `totalCount`.
  final int totalCount;
  /// Rust field `currentChatId` serialized as `currentChatId`.
  final Object? currentChatId;
  /// Rust field `chats` serialized as `chats`.
  final List<ChatInfo> chats;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::ChatMessageInfo`.
class ChatMessageInfo {
  const ChatMessageInfo({
    required this.sender,
    required this.content,
    required this.timestamp,
    required this.roleName,
    required this.provider,
    required this.modelName,
  });

  factory ChatMessageInfo.fromJson(Map<String, Object?> json) {
    return ChatMessageInfo(
      sender: json['sender'] as String,
      content: json['content'] as String,
      timestamp: json['timestamp'] as int,
      roleName: json['roleName'] as String,
      provider: json['provider'] as String,
      modelName: json['modelName'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sender': sender,
      'content': content,
      'timestamp': timestamp,
      'roleName': roleName,
      'provider': provider,
      'modelName': modelName,
    };
  }

  /// Rust field `sender` serialized as `sender`.
  final String sender;
  /// Rust field `content` serialized as `content`.
  final String content;
  /// Rust field `timestamp` serialized as `timestamp`.
  final int timestamp;
  /// Rust field `roleName` serialized as `roleName`.
  final String roleName;
  /// Rust field `provider` serialized as `provider`.
  final String provider;
  /// Rust field `modelName` serialized as `modelName`.
  final String modelName;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::ChatMessagesResultData`.
class ChatMessagesResultData {
  const ChatMessagesResultData({
    required this.chatId,
    required this.order,
    required this.limit,
    required this.messages,
  });

  factory ChatMessagesResultData.fromJson(Map<String, Object?> json) {
    return ChatMessagesResultData(
      chatId: json['chatId'] as String,
      order: json['order'] as String,
      limit: json['limit'] as int,
      messages: (json['messages'] as List<Object?>).map((item) => ChatMessageInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'chatId': chatId,
      'order': order,
      'limit': limit,
      'messages': messages.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `chatId` serialized as `chatId`.
  final String chatId;
  /// Rust field `order` serialized as `order`.
  final String order;
  /// Rust field `limit` serialized as `limit`.
  final int limit;
  /// Rust field `messages` serialized as `messages`.
  final List<ChatMessageInfo> messages;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::ChatServiceStartResultData`.
class ChatServiceStartResultData {
  const ChatServiceStartResultData({
    required this.isConnected,
    required this.connectionTime,
  });

  factory ChatServiceStartResultData.fromJson(Map<String, Object?> json) {
    return ChatServiceStartResultData(
      isConnected: json['isConnected'] as bool,
      connectionTime: json['connectionTime'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'isConnected': isConnected,
      'connectionTime': connectionTime,
    };
  }

  /// Rust field `isConnected` serialized as `isConnected`.
  final bool isConnected;
  /// Rust field `connectionTime` serialized as `connectionTime`.
  final int connectionTime;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::ChatSwitchResultData`.
class ChatSwitchResultData {
  const ChatSwitchResultData({
    required this.chatId,
    required this.chatTitle,
    required this.switchedAt,
  });

  factory ChatSwitchResultData.fromJson(Map<String, Object?> json) {
    return ChatSwitchResultData(
      chatId: json['chatId'] as String,
      chatTitle: json['chatTitle'] as String,
      switchedAt: json['switchedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'chatId': chatId,
      'chatTitle': chatTitle,
      'switchedAt': switchedAt,
    };
  }

  /// Rust field `chatId` serialized as `chatId`.
  final String chatId;
  /// Rust field `chatTitle` serialized as `chatTitle`.
  final String chatTitle;
  /// Rust field `switchedAt` serialized as `switchedAt`.
  final int switchedAt;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::ChatTitleUpdateResultData`.
class ChatTitleUpdateResultData {
  const ChatTitleUpdateResultData({
    required this.chatId,
    required this.title,
    required this.updatedAt,
  });

  factory ChatTitleUpdateResultData.fromJson(Map<String, Object?> json) {
    return ChatTitleUpdateResultData(
      chatId: json['chatId'] as String,
      title: json['title'] as String,
      updatedAt: json['updatedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'chatId': chatId,
      'title': title,
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `chatId` serialized as `chatId`.
  final String chatId;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `updatedAt` serialized as `updatedAt`.
  final int updatedAt;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::DeviceInfoResultData`.
class DeviceInfoResultData {
  const DeviceInfoResultData({
    required this.deviceId,
    required this.model,
    required this.manufacturer,
    required this.androidVersion,
    required this.sdkVersion,
    required this.screenResolution,
    required this.screenDensity,
    required this.totalMemory,
    required this.availableMemory,
    required this.totalStorage,
    required this.availableStorage,
    required this.batteryLevel,
    required this.batteryCharging,
    required this.cpuInfo,
    required this.networkType,
    required this.additionalInfo,
  });

  factory DeviceInfoResultData.fromJson(Map<String, Object?> json) {
    return DeviceInfoResultData(
      deviceId: json['deviceId'] as String,
      model: json['model'] as String,
      manufacturer: json['manufacturer'] as String,
      androidVersion: json['androidVersion'] as String,
      sdkVersion: json['sdkVersion'] as int,
      screenResolution: json['screenResolution'] as String,
      screenDensity: (json['screenDensity'] as num).toDouble(),
      totalMemory: json['totalMemory'] as String,
      availableMemory: json['availableMemory'] as String,
      totalStorage: json['totalStorage'] as String,
      availableStorage: json['availableStorage'] as String,
      batteryLevel: json['batteryLevel'] as int,
      batteryCharging: json['batteryCharging'] as bool,
      cpuInfo: json['cpuInfo'] as String,
      networkType: json['networkType'] as String,
      additionalInfo: (json['additionalInfo'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'deviceId': deviceId,
      'model': model,
      'manufacturer': manufacturer,
      'androidVersion': androidVersion,
      'sdkVersion': sdkVersion,
      'screenResolution': screenResolution,
      'screenDensity': screenDensity,
      'totalMemory': totalMemory,
      'availableMemory': availableMemory,
      'totalStorage': totalStorage,
      'availableStorage': availableStorage,
      'batteryLevel': batteryLevel,
      'batteryCharging': batteryCharging,
      'cpuInfo': cpuInfo,
      'networkType': networkType,
      'additionalInfo': additionalInfo.map((key, value) => MapEntry(key, value)),
    };
  }

  /// Rust field `deviceId` serialized as `deviceId`.
  final String deviceId;
  /// Rust field `model` serialized as `model`.
  final String model;
  /// Rust field `manufacturer` serialized as `manufacturer`.
  final String manufacturer;
  /// Rust field `androidVersion` serialized as `androidVersion`.
  final String androidVersion;
  /// Rust field `sdkVersion` serialized as `sdkVersion`.
  final int sdkVersion;
  /// Rust field `screenResolution` serialized as `screenResolution`.
  final String screenResolution;
  /// Rust field `screenDensity` serialized as `screenDensity`.
  final double screenDensity;
  /// Rust field `totalMemory` serialized as `totalMemory`.
  final String totalMemory;
  /// Rust field `availableMemory` serialized as `availableMemory`.
  final String availableMemory;
  /// Rust field `totalStorage` serialized as `totalStorage`.
  final String totalStorage;
  /// Rust field `availableStorage` serialized as `availableStorage`.
  final String availableStorage;
  /// Rust field `batteryLevel` serialized as `batteryLevel`.
  final int batteryLevel;
  /// Rust field `batteryCharging` serialized as `batteryCharging`.
  final bool batteryCharging;
  /// Rust field `cpuInfo` serialized as `cpuInfo`.
  final String cpuInfo;
  /// Rust field `networkType` serialized as `networkType`.
  final String networkType;
  /// Rust field `additionalInfo` serialized as `additionalInfo`.
  final Map<String, String> additionalInfo;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::DirectoryListingData`.
class DirectoryListingData {
  const DirectoryListingData({
    required this.path,
    required this.entries,
  });

  factory DirectoryListingData.fromJson(Map<String, Object?> json) {
    return DirectoryListingData(
      path: json['path'] as String,
      entries: (json['entries'] as List<Object?>).map((item) => FileEntry.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'entries': entries.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `entries` serialized as `entries`.
  final List<FileEntry> entries;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::EnvironmentVariableReadResultData`.
class EnvironmentVariableReadResultData {
  const EnvironmentVariableReadResultData({
    required this.key,
    required this.value,
    required this.exists,
  });

  factory EnvironmentVariableReadResultData.fromJson(Map<String, Object?> json) {
    return EnvironmentVariableReadResultData(
      key: json['key'] as String,
      value: json['value'],
      exists: json['exists'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'key': key,
      'value': value,
      'exists': exists,
    };
  }

  /// Rust field `key` serialized as `key`.
  final String key;
  /// Rust field `value` serialized as `value`.
  final Object? value;
  /// Rust field `exists` serialized as `exists`.
  final bool exists;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::EnvironmentVariableWriteResultData`.
class EnvironmentVariableWriteResultData {
  const EnvironmentVariableWriteResultData({
    required this.key,
    required this.requestedValue,
    required this.value,
    required this.exists,
    required this.cleared,
  });

  factory EnvironmentVariableWriteResultData.fromJson(Map<String, Object?> json) {
    return EnvironmentVariableWriteResultData(
      key: json['key'] as String,
      requestedValue: json['requestedValue'] as String,
      value: json['value'],
      exists: json['exists'] as bool,
      cleared: json['cleared'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'key': key,
      'requestedValue': requestedValue,
      'value': value,
      'exists': exists,
      'cleared': cleared,
    };
  }

  /// Rust field `key` serialized as `key`.
  final String key;
  /// Rust field `requestedValue` serialized as `requestedValue`.
  final String requestedValue;
  /// Rust field `value` serialized as `value`.
  final Object? value;
  /// Rust field `exists` serialized as `exists`.
  final bool exists;
  /// Rust field `cleared` serialized as `cleared`.
  final bool cleared;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::FileApplyResultData`.
class FileApplyResultData {
  const FileApplyResultData({
    required this.operation,
    required this.aiDiffInstructions,
    required this.diffContent,
  });

  factory FileApplyResultData.fromJson(Map<String, Object?> json) {
    return FileApplyResultData(
      operation: FileOperationData.fromJson(json['operation'] as Map<String, Object?>),
      aiDiffInstructions: json['aiDiffInstructions'] as String,
      diffContent: json['diffContent'] == null ? null : json['diffContent'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'operation': operation.toJson(),
      'aiDiffInstructions': aiDiffInstructions,
      'diffContent': diffContent,
    };
  }

  /// Rust field `operation` serialized as `operation`.
  final FileOperationData operation;
  /// Rust field `aiDiffInstructions` serialized as `aiDiffInstructions`.
  final String aiDiffInstructions;
  /// Rust field `diffContent` serialized as `diffContent`.
  final String? diffContent;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::FileContentData`.
class FileContentData {
  const FileContentData({
    required this.path,
    required this.content,
    required this.size,
  });

  factory FileContentData.fromJson(Map<String, Object?> json) {
    return FileContentData(
      path: json['path'] as String,
      content: json['content'] as String,
      size: json['size'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'content': content,
      'size': size,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `content` serialized as `content`.
  final String content;
  /// Rust field `size` serialized as `size`.
  final int size;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::FileEntry`.
class FileEntry {
  const FileEntry({
    required this.name,
    required this.isDirectory,
    required this.size,
    required this.permissions,
    required this.lastModified,
  });

  factory FileEntry.fromJson(Map<String, Object?> json) {
    return FileEntry(
      name: json['name'] as String,
      isDirectory: json['isDirectory'] as bool,
      size: json['size'] as int,
      permissions: json['permissions'] as String,
      lastModified: json['lastModified'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'isDirectory': isDirectory,
      'size': size,
      'permissions': permissions,
      'lastModified': lastModified,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `isDirectory` serialized as `isDirectory`.
  final bool isDirectory;
  /// Rust field `size` serialized as `size`.
  final int size;
  /// Rust field `permissions` serialized as `permissions`.
  final String permissions;
  /// Rust field `lastModified` serialized as `lastModified`.
  final String lastModified;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::FileExistsData`.
class FileExistsData {
  const FileExistsData({
    required this.path,
    required this.exists,
    required this.isDirectory,
    required this.size,
  });

  factory FileExistsData.fromJson(Map<String, Object?> json) {
    return FileExistsData(
      path: json['path'] as String,
      exists: json['exists'] as bool,
      isDirectory: json['isDirectory'] as bool,
      size: json['size'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'exists': exists,
      'isDirectory': isDirectory,
      'size': size,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `exists` serialized as `exists`.
  final bool exists;
  /// Rust field `isDirectory` serialized as `isDirectory`.
  final bool isDirectory;
  /// Rust field `size` serialized as `size`.
  final int size;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::FileInfoData`.
class FileInfoData {
  const FileInfoData({
    required this.path,
    required this.exists,
    required this.fileType,
    required this.size,
    required this.permissions,
    required this.owner,
    required this.group,
    required this.lastModified,
    required this.rawStatOutput,
  });

  factory FileInfoData.fromJson(Map<String, Object?> json) {
    return FileInfoData(
      path: json['path'] as String,
      exists: json['exists'] as bool,
      fileType: json['fileType'] as String,
      size: json['size'] as int,
      permissions: json['permissions'] as String,
      owner: json['owner'] as String,
      group: json['group'] as String,
      lastModified: json['lastModified'] as String,
      rawStatOutput: json['rawStatOutput'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'exists': exists,
      'fileType': fileType,
      'size': size,
      'permissions': permissions,
      'owner': owner,
      'group': group,
      'lastModified': lastModified,
      'rawStatOutput': rawStatOutput,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `exists` serialized as `exists`.
  final bool exists;
  /// Rust field `fileType` serialized as `fileType`.
  final String fileType;
  /// Rust field `size` serialized as `size`.
  final int size;
  /// Rust field `permissions` serialized as `permissions`.
  final String permissions;
  /// Rust field `owner` serialized as `owner`.
  final String owner;
  /// Rust field `group` serialized as `group`.
  final String group;
  /// Rust field `lastModified` serialized as `lastModified`.
  final String lastModified;
  /// Rust field `rawStatOutput` serialized as `rawStatOutput`.
  final String rawStatOutput;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::FileOperationData`.
class FileOperationData {
  const FileOperationData({
    required this.operation,
    required this.path,
    required this.successful,
    required this.details,
  });

  factory FileOperationData.fromJson(Map<String, Object?> json) {
    return FileOperationData(
      operation: json['operation'] as String,
      path: json['path'] as String,
      successful: json['successful'] as bool,
      details: json['details'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'operation': operation,
      'path': path,
      'successful': successful,
      'details': details,
    };
  }

  /// Rust field `operation` serialized as `operation`.
  final String operation;
  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `successful` serialized as `successful`.
  final bool successful;
  /// Rust field `details` serialized as `details`.
  final String details;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::FilePartContentData`.
class FilePartContentData {
  const FilePartContentData({
    required this.path,
    required this.content,
    required this.partIndex,
    required this.totalParts,
    required this.startLine,
    required this.endLine,
    required this.totalLines,
  });

  factory FilePartContentData.fromJson(Map<String, Object?> json) {
    return FilePartContentData(
      path: json['path'] as String,
      content: json['content'] as String,
      partIndex: json['partIndex'] as int,
      totalParts: json['totalParts'] as int,
      startLine: json['startLine'] as int,
      endLine: json['endLine'] as int,
      totalLines: json['totalLines'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'content': content,
      'partIndex': partIndex,
      'totalParts': totalParts,
      'startLine': startLine,
      'endLine': endLine,
      'totalLines': totalLines,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `content` serialized as `content`.
  final String content;
  /// Rust field `partIndex` serialized as `partIndex`.
  final int partIndex;
  /// Rust field `totalParts` serialized as `totalParts`.
  final int totalParts;
  /// Rust field `startLine` serialized as `startLine`.
  final int startLine;
  /// Rust field `endLine` serialized as `endLine`.
  final int endLine;
  /// Rust field `totalLines` serialized as `totalLines`.
  final int totalLines;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::FindFilesResultData`.
class FindFilesResultData {
  const FindFilesResultData({
    required this.path,
    required this.pattern,
    required this.files,
  });

  factory FindFilesResultData.fromJson(Map<String, Object?> json) {
    return FindFilesResultData(
      path: json['path'] as String,
      pattern: json['pattern'] as String,
      files: (json['files'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'pattern': pattern,
      'files': files.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `pattern` serialized as `pattern`.
  final String pattern;
  /// Rust field `files` serialized as `files`.
  final List<String> files;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::GrepFileMatch`.
class GrepFileMatch {
  const GrepFileMatch({
    required this.filePath,
    required this.lineMatches,
  });

  factory GrepFileMatch.fromJson(Map<String, Object?> json) {
    return GrepFileMatch(
      filePath: json['filePath'] as String,
      lineMatches: (json['lineMatches'] as List<Object?>).map((item) => GrepLineMatch.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'filePath': filePath,
      'lineMatches': lineMatches.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `filePath` serialized as `filePath`.
  final String filePath;
  /// Rust field `lineMatches` serialized as `lineMatches`.
  final List<GrepLineMatch> lineMatches;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::GrepLineMatch`.
class GrepLineMatch {
  const GrepLineMatch({
    required this.lineNumber,
    required this.lineContent,
    required this.matchContext,
  });

  factory GrepLineMatch.fromJson(Map<String, Object?> json) {
    return GrepLineMatch(
      lineNumber: json['lineNumber'] as int,
      lineContent: json['lineContent'] as String,
      matchContext: json['matchContext'] == null ? null : json['matchContext'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'lineNumber': lineNumber,
      'lineContent': lineContent,
      'matchContext': matchContext,
    };
  }

  /// Rust field `lineNumber` serialized as `lineNumber`.
  final int lineNumber;
  /// Rust field `lineContent` serialized as `lineContent`.
  final String lineContent;
  /// Rust field `matchContext` serialized as `matchContext`.
  final String? matchContext;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::GrepResultData`.
class GrepResultData {
  const GrepResultData({
    required this.searchPath,
    required this.pattern,
    required this.matches,
    required this.totalMatches,
    required this.filesSearched,
  });

  factory GrepResultData.fromJson(Map<String, Object?> json) {
    return GrepResultData(
      searchPath: json['searchPath'] as String,
      pattern: json['pattern'] as String,
      matches: (json['matches'] as List<Object?>).map((item) => GrepFileMatch.fromJson(item as Map<String, Object?>)).toList(growable: false),
      totalMatches: json['totalMatches'] as int,
      filesSearched: json['filesSearched'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'searchPath': searchPath,
      'pattern': pattern,
      'matches': matches.map((item) => item.toJson()).toList(growable: false),
      'totalMatches': totalMatches,
      'filesSearched': filesSearched,
    };
  }

  /// Rust field `searchPath` serialized as `searchPath`.
  final String searchPath;
  /// Rust field `pattern` serialized as `pattern`.
  final String pattern;
  /// Rust field `matches` serialized as `matches`.
  final List<GrepFileMatch> matches;
  /// Rust field `totalMatches` serialized as `totalMatches`.
  final int totalMatches;
  /// Rust field `filesSearched` serialized as `filesSearched`.
  final int filesSearched;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::HiddenTerminalCommandResultData`.
class HiddenTerminalCommandResultData {
  const HiddenTerminalCommandResultData({
    required this.command,
    required this.output,
    required this.exitCode,
    required this.executorKey,
    required this.platform,
    required this.terminal,
    required this.terminalType,
    required this.timedOut,
  });

  factory HiddenTerminalCommandResultData.fromJson(Map<String, Object?> json) {
    return HiddenTerminalCommandResultData(
      command: json['command'] as String,
      output: json['output'] as String,
      exitCode: json['exitCode'] as int,
      executorKey: json['executorKey'] as String,
      platform: json['platform'] as String,
      terminal: TerminalImplementation.fromJson(json['terminal']),
      terminalType: TerminalType.fromJson(json['terminalType']),
      timedOut: json['timedOut'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'command': command,
      'output': output,
      'exitCode': exitCode,
      'executorKey': executorKey,
      'platform': platform,
      'terminal': terminal.toJson(),
      'terminalType': terminalType.toJson(),
      'timedOut': timedOut,
    };
  }

  /// Rust field `command` serialized as `command`.
  final String command;
  /// Rust field `output` serialized as `output`.
  final String output;
  /// Rust field `exitCode` serialized as `exitCode`.
  final int exitCode;
  /// Rust field `executorKey` serialized as `executorKey`.
  final String executorKey;
  /// Rust field `platform` serialized as `platform`.
  final String platform;
  /// Rust field `terminal` serialized as `terminal`.
  final TerminalImplementation terminal;
  /// Rust field `terminalType` serialized as `terminalType`.
  final TerminalType terminalType;
  /// Rust field `timedOut` serialized as `timedOut`.
  final bool timedOut;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::HttpResponseData`.
class HttpResponseData {
  const HttpResponseData({
    required this.url,
    required this.statusCode,
    required this.statusMessage,
    required this.headers,
    required this.contentType,
    required this.content,
    required this.contentBase64,
    required this.size,
    required this.cookies,
  });

  factory HttpResponseData.fromJson(Map<String, Object?> json) {
    return HttpResponseData(
      url: json['url'] as String,
      statusCode: json['statusCode'] as int,
      statusMessage: json['statusMessage'] as String,
      headers: (json['headers'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
      contentType: json['contentType'] as String,
      content: json['content'] as String,
      contentBase64: json['contentBase64'] == null ? null : json['contentBase64'] as String,
      size: json['size'] as int,
      cookies: (json['cookies'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'url': url,
      'statusCode': statusCode,
      'statusMessage': statusMessage,
      'headers': headers.map((key, value) => MapEntry(key, value)),
      'contentType': contentType,
      'content': content,
      'contentBase64': contentBase64,
      'size': size,
      'cookies': cookies.map((key, value) => MapEntry(key, value)),
    };
  }

  /// Rust field `url` serialized as `url`.
  final String url;
  /// Rust field `statusCode` serialized as `statusCode`.
  final int statusCode;
  /// Rust field `statusMessage` serialized as `statusMessage`.
  final String statusMessage;
  /// Rust field `headers` serialized as `headers`.
  final Map<String, String> headers;
  /// Rust field `contentType` serialized as `contentType`.
  final String contentType;
  /// Rust field `content` serialized as `content`.
  final String content;
  /// Rust field `contentBase64` serialized as `contentBase64`.
  final String? contentBase64;
  /// Rust field `size` serialized as `size`.
  final int size;
  /// Rust field `cookies` serialized as `cookies`.
  final Map<String, String> cookies;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::HttpStreamEventData`.
class HttpStreamEventData {
  const HttpStreamEventData({
    required this.type,
    required this.url,
    required this.statusCode,
    required this.statusMessage,
    required this.headers,
    required this.contentType,
    required this.chunk,
    required this.chunkIndex,
    required this.receivedBytes,
  });

  factory HttpStreamEventData.fromJson(Map<String, Object?> json) {
    return HttpStreamEventData(
      type: json['type'] as String,
      url: json['url'] as String,
      statusCode: json['statusCode'] == null ? null : json['statusCode'] as int,
      statusMessage: json['statusMessage'] == null ? null : json['statusMessage'] as String,
      headers: (json['headers'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
      contentType: json['contentType'] == null ? null : json['contentType'] as String,
      chunk: json['chunk'] == null ? null : json['chunk'] as String,
      chunkIndex: json['chunkIndex'] == null ? null : json['chunkIndex'] as int,
      receivedBytes: json['receivedBytes'] == null ? null : json['receivedBytes'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'type': type,
      'url': url,
      'statusCode': statusCode,
      'statusMessage': statusMessage,
      'headers': headers.map((key, value) => MapEntry(key, value)),
      'contentType': contentType,
      'chunk': chunk,
      'chunkIndex': chunkIndex,
      'receivedBytes': receivedBytes,
    };
  }

  /// Rust field `r#type` serialized as `type`.
  final String type;
  /// Rust field `url` serialized as `url`.
  final String url;
  /// Rust field `statusCode` serialized as `statusCode`.
  final int? statusCode;
  /// Rust field `statusMessage` serialized as `statusMessage`.
  final String? statusMessage;
  /// Rust field `headers` serialized as `headers`.
  final Map<String, String> headers;
  /// Rust field `contentType` serialized as `contentType`.
  final String? contentType;
  /// Rust field `chunk` serialized as `chunk`.
  final String? chunk;
  /// Rust field `chunkIndex` serialized as `chunkIndex`.
  final int? chunkIndex;
  /// Rust field `receivedBytes` serialized as `receivedBytes`.
  final int? receivedBytes;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::IntResultData`.
class IntResultData {
  const IntResultData({
    required this.value,
  });

  factory IntResultData.fromJson(Map<String, Object?> json) {
    return IntResultData(
      value: json['value'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'value': value,
    };
  }

  /// Rust field `value` serialized as `value`.
  final int value;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::LinkData`.
class LinkData {
  const LinkData({
    required this.url,
    required this.text,
  });

  factory LinkData.fromJson(Map<String, Object?> json) {
    return LinkData(
      url: json['url'] as String,
      text: json['text'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'url': url,
      'text': text,
    };
  }

  /// Rust field `url` serialized as `url`.
  final String url;
  /// Rust field `text` serialized as `text`.
  final String text;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::LinkInfo`.
class LinkInfo {
  const LinkInfo({
    required this.linkId,
    required this.sourceTitle,
    required this.targetTitle,
    required this.linkType,
    required this.weight,
    required this.description,
  });

  factory LinkInfo.fromJson(Map<String, Object?> json) {
    return LinkInfo(
      linkId: json['linkId'] as int,
      sourceTitle: json['sourceTitle'] as String,
      targetTitle: json['targetTitle'] as String,
      linkType: json['linkType'] as String,
      weight: (json['weight'] as num).toDouble(),
      description: json['description'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'linkId': linkId,
      'sourceTitle': sourceTitle,
      'targetTitle': targetTitle,
      'linkType': linkType,
      'weight': weight,
      'description': description,
    };
  }

  /// Rust field `linkId` serialized as `linkId`.
  final int linkId;
  /// Rust field `sourceTitle` serialized as `sourceTitle`.
  final String sourceTitle;
  /// Rust field `targetTitle` serialized as `targetTitle`.
  final String targetTitle;
  /// Rust field `linkType` serialized as `linkType`.
  final String linkType;
  /// Rust field `weight` serialized as `weight`.
  final double weight;
  /// Rust field `description` serialized as `description`.
  final String description;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::LocationData`.
class LocationData {
  const LocationData({
    required this.latitude,
    required this.longitude,
    required this.accuracy,
    required this.provider,
    required this.timestamp,
    required this.rawData,
    required this.address,
    required this.city,
    required this.province,
    required this.country,
  });

  factory LocationData.fromJson(Map<String, Object?> json) {
    return LocationData(
      latitude: (json['latitude'] as num).toDouble(),
      longitude: (json['longitude'] as num).toDouble(),
      accuracy: (json['accuracy'] as num).toDouble(),
      provider: json['provider'] as String,
      timestamp: json['timestamp'] as int,
      rawData: json['rawData'] as String,
      address: json['address'] as String,
      city: json['city'] as String,
      province: json['province'] as String,
      country: json['country'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'latitude': latitude,
      'longitude': longitude,
      'accuracy': accuracy,
      'provider': provider,
      'timestamp': timestamp,
      'rawData': rawData,
      'address': address,
      'city': city,
      'province': province,
      'country': country,
    };
  }

  /// Rust field `latitude` serialized as `latitude`.
  final double latitude;
  /// Rust field `longitude` serialized as `longitude`.
  final double longitude;
  /// Rust field `accuracy` serialized as `accuracy`.
  final double accuracy;
  /// Rust field `provider` serialized as `provider`.
  final String provider;
  /// Rust field `timestamp` serialized as `timestamp`.
  final int timestamp;
  /// Rust field `rawData` serialized as `rawData`.
  final String rawData;
  /// Rust field `address` serialized as `address`.
  final String address;
  /// Rust field `city` serialized as `city`.
  final String city;
  /// Rust field `province` serialized as `province`.
  final String province;
  /// Rust field `country` serialized as `country`.
  final String country;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::MemoryInfo`.
class MemoryInfo {
  const MemoryInfo({
    required this.ownerKey,
    required this.title,
    required this.content,
    required this.source,
    required this.tags,
    required this.createdAt,
    required this.chunkInfo,
    required this.chunkIndices,
  });

  factory MemoryInfo.fromJson(Map<String, Object?> json) {
    return MemoryInfo(
      ownerKey: json['ownerKey'] as String,
      title: json['title'] as String,
      content: json['content'] as String,
      source: json['source'] as String,
      tags: (json['tags'] as List<Object?>).map((item) => item as String).toList(growable: false),
      createdAt: json['createdAt'] as String,
      chunkInfo: json['chunkInfo'],
      chunkIndices: json['chunkIndices'],
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'ownerKey': ownerKey,
      'title': title,
      'content': content,
      'source': source,
      'tags': tags.map((item) => item).toList(growable: false),
      'createdAt': createdAt,
      'chunkInfo': chunkInfo,
      'chunkIndices': chunkIndices,
    };
  }

  /// Rust field `ownerKey` serialized as `ownerKey`.
  final String ownerKey;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `content` serialized as `content`.
  final String content;
  /// Rust field `source` serialized as `source`.
  final String source;
  /// Rust field `tags` serialized as `tags`.
  final List<String> tags;
  /// Rust field `createdAt` serialized as `createdAt`.
  final String createdAt;
  /// Rust field `chunkInfo` serialized as `chunkInfo`.
  final Object? chunkInfo;
  /// Rust field `chunkIndices` serialized as `chunkIndices`.
  final Object? chunkIndices;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::MemoryLinkQueryResultData`.
class MemoryLinkQueryResultData {
  const MemoryLinkQueryResultData({
    required this.totalCount,
    required this.links,
  });

  factory MemoryLinkQueryResultData.fromJson(Map<String, Object?> json) {
    return MemoryLinkQueryResultData(
      totalCount: json['totalCount'] as int,
      links: (json['links'] as List<Object?>).map((item) => LinkInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'totalCount': totalCount,
      'links': links.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `totalCount` serialized as `totalCount`.
  final int totalCount;
  /// Rust field `links` serialized as `links`.
  final List<LinkInfo> links;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::MemoryLinkResultData`.
class MemoryLinkResultData {
  const MemoryLinkResultData({
    required this.sourceTitle,
    required this.targetTitle,
    required this.linkType,
    required this.weight,
    required this.description,
  });

  factory MemoryLinkResultData.fromJson(Map<String, Object?> json) {
    return MemoryLinkResultData(
      sourceTitle: json['sourceTitle'] as String,
      targetTitle: json['targetTitle'] as String,
      linkType: json['linkType'] as String,
      weight: (json['weight'] as num).toDouble(),
      description: json['description'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sourceTitle': sourceTitle,
      'targetTitle': targetTitle,
      'linkType': linkType,
      'weight': weight,
      'description': description,
    };
  }

  /// Rust field `sourceTitle` serialized as `sourceTitle`.
  final String sourceTitle;
  /// Rust field `targetTitle` serialized as `targetTitle`.
  final String targetTitle;
  /// Rust field `linkType` serialized as `linkType`.
  final String linkType;
  /// Rust field `weight` serialized as `weight`.
  final double weight;
  /// Rust field `description` serialized as `description`.
  final String description;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::MemoryQueryResultData`.
class MemoryQueryResultData {
  const MemoryQueryResultData({
    required this.memories,
    required this.snapshotId,
    required this.snapshotCreated,
    required this.excludedBySnapshotCount,
  });

  factory MemoryQueryResultData.fromJson(Map<String, Object?> json) {
    return MemoryQueryResultData(
      memories: (json['memories'] as List<Object?>).map((item) => MemoryInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
      snapshotId: json['snapshotId'],
      snapshotCreated: json['snapshotCreated'] == null ? null : json['snapshotCreated'] as bool,
      excludedBySnapshotCount: json['excludedBySnapshotCount'] == null ? null : json['excludedBySnapshotCount'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'memories': memories.map((item) => item.toJson()).toList(growable: false),
      'snapshotId': snapshotId,
      'snapshotCreated': snapshotCreated,
      'excludedBySnapshotCount': excludedBySnapshotCount,
    };
  }

  /// Rust field `memories` serialized as `memories`.
  final List<MemoryInfo> memories;
  /// Rust field `snapshotId` serialized as `snapshotId`.
  final Object? snapshotId;
  /// Rust field `snapshotCreated` serialized as `snapshotCreated`.
  final bool? snapshotCreated;
  /// Rust field `excludedBySnapshotCount` serialized as `excludedBySnapshotCount`.
  final int? excludedBySnapshotCount;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::MessageSendResultData`.
class MessageSendResultData {
  const MessageSendResultData({
    required this.chatId,
    required this.message,
    required this.aiResponse,
    required this.receivedAt,
    required this.sentAt,
  });

  factory MessageSendResultData.fromJson(Map<String, Object?> json) {
    return MessageSendResultData(
      chatId: json['chatId'] as String,
      message: json['message'] as String,
      aiResponse: json['aiResponse'],
      receivedAt: json['receivedAt'],
      sentAt: json['sentAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'chatId': chatId,
      'message': message,
      'aiResponse': aiResponse,
      'receivedAt': receivedAt,
      'sentAt': sentAt,
    };
  }

  /// Rust field `chatId` serialized as `chatId`.
  final String chatId;
  /// Rust field `message` serialized as `message`.
  final String message;
  /// Rust field `aiResponse` serialized as `aiResponse`.
  final Object? aiResponse;
  /// Rust field `receivedAt` serialized as `receivedAt`.
  final Object? receivedAt;
  /// Rust field `sentAt` serialized as `sentAt`.
  final int sentAt;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::MusicPlaybackResultData`.
class MusicPlaybackResultData {
  const MusicPlaybackResultData({
    required this.state,
    required this.source,
    required this.sourceType,
    required this.title,
    required this.artist,
    required this.durationMs,
    required this.positionMs,
    required this.bufferedPositionMs,
    required this.volume,
    required this.loop,
    required this.message,
  });

  factory MusicPlaybackResultData.fromJson(Map<String, Object?> json) {
    return MusicPlaybackResultData(
      state: json['state'] as String,
      source: json['source'],
      sourceType: json['sourceType'],
      title: json['title'],
      artist: json['artist'],
      durationMs: json['durationMs'],
      positionMs: json['positionMs'] as int,
      bufferedPositionMs: json['bufferedPositionMs'] as int,
      volume: (json['volume'] as num).toDouble(),
      loop: json['loop'] as bool,
      message: json['message'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'state': state,
      'source': source,
      'sourceType': sourceType,
      'title': title,
      'artist': artist,
      'durationMs': durationMs,
      'positionMs': positionMs,
      'bufferedPositionMs': bufferedPositionMs,
      'volume': volume,
      'loop': loop,
      'message': message,
    };
  }

  /// Rust field `state` serialized as `state`.
  final String state;
  /// Rust field `source` serialized as `source`.
  final Object? source;
  /// Rust field `sourceType` serialized as `sourceType`.
  final Object? sourceType;
  /// Rust field `title` serialized as `title`.
  final Object? title;
  /// Rust field `artist` serialized as `artist`.
  final Object? artist;
  /// Rust field `durationMs` serialized as `durationMs`.
  final Object? durationMs;
  /// Rust field `positionMs` serialized as `positionMs`.
  final int positionMs;
  /// Rust field `bufferedPositionMs` serialized as `bufferedPositionMs`.
  final int bufferedPositionMs;
  /// Rust field `volume` serialized as `volume`.
  final double volume;
  /// Rust field `r#loop` serialized as `loop`.
  final bool loop;
  /// Rust field `message` serialized as `message`.
  final String message;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::Notification`.
class Notification {
  const Notification({
    required this.packageName,
    required this.text,
    required this.timestamp,
  });

  factory Notification.fromJson(Map<String, Object?> json) {
    return Notification(
      packageName: json['packageName'] as String,
      text: json['text'] as String,
      timestamp: json['timestamp'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'packageName': packageName,
      'text': text,
      'timestamp': timestamp,
    };
  }

  /// Rust field `packageName` serialized as `packageName`.
  final String packageName;
  /// Rust field `text` serialized as `text`.
  final String text;
  /// Rust field `timestamp` serialized as `timestamp`.
  final int timestamp;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::NotificationData`.
class NotificationData {
  const NotificationData({
    required this.notifications,
    required this.timestamp,
  });

  factory NotificationData.fromJson(Map<String, Object?> json) {
    return NotificationData(
      notifications: (json['notifications'] as List<Object?>).map((item) => Notification.fromJson(item as Map<String, Object?>)).toList(growable: false),
      timestamp: json['timestamp'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'notifications': notifications.map((item) => item.toJson()).toList(growable: false),
      'timestamp': timestamp,
    };
  }

  /// Rust field `notifications` serialized as `notifications`.
  final List<Notification> notifications;
  /// Rust field `timestamp` serialized as `timestamp`.
  final int timestamp;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::SleepResultData`.
class SleepResultData {
  const SleepResultData({
    required this.requestedMs,
    required this.sleptMs,
  });

  factory SleepResultData.fromJson(Map<String, Object?> json) {
    return SleepResultData(
      requestedMs: json['requestedMs'] as int,
      sleptMs: json['sleptMs'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'requestedMs': requestedMs,
      'sleptMs': sleptMs,
    };
  }

  /// Rust field `requestedMs` serialized as `requestedMs`.
  final int requestedMs;
  /// Rust field `sleptMs` serialized as `sleptMs`.
  final int sleptMs;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::StringResultData`.
class StringResultData {
  const StringResultData({
    required this.value,
  });

  factory StringResultData.fromJson(Map<String, Object?> json) {
    return StringResultData(
      value: json['value'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'value': value,
    };
  }

  /// Rust field `value` serialized as `value`.
  final String value;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::SystemSettingData`.
class SystemSettingData {
  const SystemSettingData({
    required this.namespace,
    required this.setting,
    required this.value,
  });

  factory SystemSettingData.fromJson(Map<String, Object?> json) {
    return SystemSettingData(
      namespace: json['namespace'] as String,
      setting: json['setting'] as String,
      value: json['value'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'namespace': namespace,
      'setting': setting,
      'value': value,
    };
  }

  /// Rust field `namespace` serialized as `namespace`.
  final String namespace;
  /// Rust field `setting` serialized as `setting`.
  final String setting;
  /// Rust field `value` serialized as `value`.
  final String value;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::TerminalCommandResultData`.
class TerminalCommandResultData {
  const TerminalCommandResultData({
    required this.command,
    required this.output,
    required this.exitCode,
    required this.sessionId,
    required this.platform,
    required this.terminal,
    required this.terminalType,
    required this.timedOut,
  });

  factory TerminalCommandResultData.fromJson(Map<String, Object?> json) {
    return TerminalCommandResultData(
      command: json['command'] as String,
      output: json['output'] as String,
      exitCode: json['exitCode'] as int,
      sessionId: json['sessionId'] as String,
      platform: json['platform'] as String,
      terminal: TerminalImplementation.fromJson(json['terminal']),
      terminalType: TerminalType.fromJson(json['terminalType']),
      timedOut: json['timedOut'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'command': command,
      'output': output,
      'exitCode': exitCode,
      'sessionId': sessionId,
      'platform': platform,
      'terminal': terminal.toJson(),
      'terminalType': terminalType.toJson(),
      'timedOut': timedOut,
    };
  }

  /// Rust field `command` serialized as `command`.
  final String command;
  /// Rust field `output` serialized as `output`.
  final String output;
  /// Rust field `exitCode` serialized as `exitCode`.
  final int exitCode;
  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `platform` serialized as `platform`.
  final String platform;
  /// Rust field `terminal` serialized as `terminal`.
  final TerminalImplementation terminal;
  /// Rust field `terminalType` serialized as `terminalType`.
  final TerminalType terminalType;
  /// Rust field `timedOut` serialized as `timedOut`.
  final bool timedOut;
}

/// Generated Dart enum for Rust type `operit_plugin_sdk::js_sdk::results::TerminalImplementation`.
enum TerminalImplementation {
  native('native'),
  proot('proot'),
  androidSystem('android-system'),
  adb('adb'),
  shell('shell'),
  ish('ish'),
  qemuVroot('qemu-vroot'),
  v86('v86'),
  ;

  const TerminalImplementation(this.value);

  final String value;

  factory TerminalImplementation.fromJson(Object? value) {
    return switch (value) {
      'native' => TerminalImplementation.native,
      'proot' => TerminalImplementation.proot,
      'android-system' => TerminalImplementation.androidSystem,
      'adb' => TerminalImplementation.adb,
      'shell' => TerminalImplementation.shell,
      'ish' => TerminalImplementation.ish,
      'qemu-vroot' => TerminalImplementation.qemuVroot,
      'v86' => TerminalImplementation.v86,
      _ => throw ArgumentError('Unknown TerminalImplementation: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::TerminalInfoResultData`.
class TerminalInfoResultData {
  const TerminalInfoResultData({
    required this.platform,
    required this.terminal,
    required this.terminalType,
    required this.types,
  });

  factory TerminalInfoResultData.fromJson(Map<String, Object?> json) {
    return TerminalInfoResultData(
      platform: json['platform'] as String,
      terminal: TerminalImplementation.fromJson(json['terminal']),
      terminalType: TerminalType.fromJson(json['terminalType']),
      types: (json['types'] as List<Object?>).map((item) => TerminalTypeInfoData.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'platform': platform,
      'terminal': terminal.toJson(),
      'terminalType': terminalType.toJson(),
      'types': types.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `platform` serialized as `platform`.
  final String platform;
  /// Rust field `terminal` serialized as `terminal`.
  final TerminalImplementation terminal;
  /// Rust field `terminalType` serialized as `terminalType`.
  final TerminalType terminalType;
  /// Rust field `types` serialized as `types`.
  final List<TerminalTypeInfoData> types;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::TerminalSessionCloseResultData`.
class TerminalSessionCloseResultData {
  const TerminalSessionCloseResultData({
    required this.sessionId,
    required this.success,
    required this.message,
  });

  factory TerminalSessionCloseResultData.fromJson(Map<String, Object?> json) {
    return TerminalSessionCloseResultData(
      sessionId: json['sessionId'] as String,
      success: json['success'] as bool,
      message: json['message'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'success': success,
      'message': message,
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `success` serialized as `success`.
  final bool success;
  /// Rust field `message` serialized as `message`.
  final String message;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::TerminalSessionCreationResultData`.
class TerminalSessionCreationResultData {
  const TerminalSessionCreationResultData({
    required this.sessionId,
    required this.sessionName,
    required this.platform,
    required this.terminal,
    required this.terminalType,
    required this.isNewSession,
  });

  factory TerminalSessionCreationResultData.fromJson(Map<String, Object?> json) {
    return TerminalSessionCreationResultData(
      sessionId: json['sessionId'] as String,
      sessionName: json['sessionName'] as String,
      platform: json['platform'] as String,
      terminal: TerminalImplementation.fromJson(json['terminal']),
      terminalType: TerminalType.fromJson(json['terminalType']),
      isNewSession: json['isNewSession'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'sessionName': sessionName,
      'platform': platform,
      'terminal': terminal.toJson(),
      'terminalType': terminalType.toJson(),
      'isNewSession': isNewSession,
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `sessionName` serialized as `sessionName`.
  final String sessionName;
  /// Rust field `platform` serialized as `platform`.
  final String platform;
  /// Rust field `terminal` serialized as `terminal`.
  final TerminalImplementation terminal;
  /// Rust field `terminalType` serialized as `terminalType`.
  final TerminalType terminalType;
  /// Rust field `isNewSession` serialized as `isNewSession`.
  final bool isNewSession;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::TerminalSessionScreenResultData`.
class TerminalSessionScreenResultData {
  const TerminalSessionScreenResultData({
    required this.sessionId,
    required this.platform,
    required this.terminal,
    required this.terminalType,
    required this.rows,
    required this.cols,
    required this.content,
    required this.commandRunning,
  });

  factory TerminalSessionScreenResultData.fromJson(Map<String, Object?> json) {
    return TerminalSessionScreenResultData(
      sessionId: json['sessionId'] as String,
      platform: json['platform'] as String,
      terminal: TerminalImplementation.fromJson(json['terminal']),
      terminalType: TerminalType.fromJson(json['terminalType']),
      rows: json['rows'] as int,
      cols: json['cols'] as int,
      content: json['content'] as String,
      commandRunning: json['commandRunning'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'platform': platform,
      'terminal': terminal.toJson(),
      'terminalType': terminalType.toJson(),
      'rows': rows,
      'cols': cols,
      'content': content,
      'commandRunning': commandRunning,
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `platform` serialized as `platform`.
  final String platform;
  /// Rust field `terminal` serialized as `terminal`.
  final TerminalImplementation terminal;
  /// Rust field `terminalType` serialized as `terminalType`.
  final TerminalType terminalType;
  /// Rust field `rows` serialized as `rows`.
  final int rows;
  /// Rust field `cols` serialized as `cols`.
  final int cols;
  /// Rust field `content` serialized as `content`.
  final String content;
  /// Rust field `commandRunning` serialized as `commandRunning`.
  final bool commandRunning;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::TerminalStreamEventData`.
class TerminalStreamEventData {
  const TerminalStreamEventData({
    required this.type,
    required this.command,
    required this.sessionId,
    required this.platform,
    required this.terminal,
    required this.terminalType,
    required this.chunk,
    required this.chunkIndex,
    required this.receivedChars,
  });

  factory TerminalStreamEventData.fromJson(Map<String, Object?> json) {
    return TerminalStreamEventData(
      type: json['type'] as String,
      command: json['command'] as String,
      sessionId: json['sessionId'] as String,
      platform: json['platform'] as String,
      terminal: TerminalImplementation.fromJson(json['terminal']),
      terminalType: TerminalType.fromJson(json['terminalType']),
      chunk: json['chunk'],
      chunkIndex: json['chunkIndex'],
      receivedChars: json['receivedChars'],
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'type': type,
      'command': command,
      'sessionId': sessionId,
      'platform': platform,
      'terminal': terminal.toJson(),
      'terminalType': terminalType.toJson(),
      'chunk': chunk,
      'chunkIndex': chunkIndex,
      'receivedChars': receivedChars,
    };
  }

  /// Rust field `r#type` serialized as `type`.
  final String type;
  /// Rust field `command` serialized as `command`.
  final String command;
  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `platform` serialized as `platform`.
  final String platform;
  /// Rust field `terminal` serialized as `terminal`.
  final TerminalImplementation terminal;
  /// Rust field `terminalType` serialized as `terminalType`.
  final TerminalType terminalType;
  /// Rust field `chunk` serialized as `chunk`.
  final Object? chunk;
  /// Rust field `chunkIndex` serialized as `chunkIndex`.
  final Object? chunkIndex;
  /// Rust field `receivedChars` serialized as `receivedChars`.
  final Object? receivedChars;
}

/// Generated Dart enum for Rust type `operit_plugin_sdk::js_sdk::results::TerminalType`.
enum TerminalType {
  powershell('powershell'),
  bash('bash'),
  shell('shell'),
  ;

  const TerminalType(this.value);

  final String value;

  factory TerminalType.fromJson(Object? value) {
    return switch (value) {
      'powershell' => TerminalType.powershell,
      'bash' => TerminalType.bash,
      'shell' => TerminalType.shell,
      _ => throw ArgumentError('Unknown TerminalType: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::TerminalTypeInfoData`.
class TerminalTypeInfoData {
  const TerminalTypeInfoData({
    required this.terminal,
    required this.terminalType,
    required this.available,
    required this.description,
  });

  factory TerminalTypeInfoData.fromJson(Map<String, Object?> json) {
    return TerminalTypeInfoData(
      terminal: TerminalImplementation.fromJson(json['terminal']),
      terminalType: TerminalType.fromJson(json['terminalType']),
      available: json['available'] as bool,
      description: json['description'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'terminal': terminal.toJson(),
      'terminalType': terminalType.toJson(),
      'available': available,
      'description': description,
    };
  }

  /// Rust field `terminal` serialized as `terminal`.
  final TerminalImplementation terminal;
  /// Rust field `terminalType` serialized as `terminalType`.
  final TerminalType terminalType;
  /// Rust field `available` serialized as `available`.
  final bool available;
  /// Rust field `description` serialized as `description`.
  final String description;
}

/// Generated Dart tagged enum model for Rust type `operit_plugin_sdk::js_sdk::results::ToolResultData`.
class CoreOperitPluginSdkJsSdkResultsToolResultData {
  const CoreOperitPluginSdkJsSdkResultsToolResultData._({
    this.value = null,
    required this.tag,
  });

  factory CoreOperitPluginSdkJsSdkResultsToolResultData.booleanResultData({required BooleanResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'BooleanResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.stringResultData({required StringResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'StringResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.sleepResultData({required SleepResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'SleepResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.environmentVariableReadResultData({required EnvironmentVariableReadResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'EnvironmentVariableReadResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.environmentVariableWriteResultData({required EnvironmentVariableWriteResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'EnvironmentVariableWriteResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.intResultData({required IntResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'IntResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.binaryResultData({required BinaryResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'BinaryResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.filePartContentData({required FilePartContentData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'FilePartContentData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.directoryListingData({required DirectoryListingData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'DirectoryListingData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.fileContentData({required FileContentData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'FileContentData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.binaryFileContentData({required BinaryFileContentData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'BinaryFileContentData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.fileExistsData({required FileExistsData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'FileExistsData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.fileInfoData({required FileInfoData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'FileInfoData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.fileOperationData({required FileOperationData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'FileOperationData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.fileApplyResultData({required FileApplyResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'FileApplyResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.httpResponseData({required HttpResponseData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'HttpResponseData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.httpStreamEventData({required HttpStreamEventData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'HttpStreamEventData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.systemSettingData({required SystemSettingData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'SystemSettingData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.appOperationData({required AppOperationData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'AppOperationData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.appListData({required AppListData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'AppListData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.appUsageTimeResultData({required AppUsageTimeResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'AppUsageTimeResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.notificationData({required NotificationData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'NotificationData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.locationData({required LocationData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'LocationData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.deviceInfoResultData({required DeviceInfoResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'DeviceInfoResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.memoryQueryResultData({required MemoryQueryResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'MemoryQueryResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.chatServiceStartResultData({required ChatServiceStartResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'ChatServiceStartResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.chatCreationResultData({required ChatCreationResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'ChatCreationResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.chatListResultData({required ChatListResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'ChatListResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.chatFindResultData({required ChatFindResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'ChatFindResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.agentStatusResultData({required AgentStatusResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'AgentStatusResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.chatSwitchResultData({required ChatSwitchResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'ChatSwitchResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.chatTitleUpdateResultData({required ChatTitleUpdateResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'ChatTitleUpdateResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.chatDeleteResultData({required ChatDeleteResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'ChatDeleteResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.messageSendResultData({required MessageSendResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'MessageSendResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.chatMessagesResultData({required ChatMessagesResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'ChatMessagesResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.characterCardListResultData({required CharacterCardListResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'CharacterCardListResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.visitWebResultData({required VisitWebResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'VisitWebResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.terminalInfoResultData({required TerminalInfoResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'TerminalInfoResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.terminalCommandResultData({required TerminalCommandResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'TerminalCommandResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.terminalStreamEventData({required TerminalStreamEventData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'TerminalStreamEventData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.hiddenTerminalCommandResultData({required HiddenTerminalCommandResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'HiddenTerminalCommandResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.terminalSessionCreationResultData({required TerminalSessionCreationResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'TerminalSessionCreationResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.terminalSessionCloseResultData({required TerminalSessionCloseResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'TerminalSessionCloseResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.terminalSessionScreenResultData({required TerminalSessionScreenResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'TerminalSessionScreenResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.musicPlaybackResultData({required MusicPlaybackResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'MusicPlaybackResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothStateData({required CoreOperitPluginSdkJsSdkResultsBluetoothStateData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'BluetoothStateData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothBondedDevicesData({required CoreOperitPluginSdkJsSdkResultsBluetoothBondedDevicesData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'BluetoothBondedDevicesData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothScanResultData({required CoreOperitPluginSdkJsSdkResultsBluetoothScanResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'BluetoothScanResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothSessionData({required CoreOperitPluginSdkJsSdkResultsBluetoothSessionData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'BluetoothSessionData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothTransferData({required CoreOperitPluginSdkJsSdkResultsBluetoothTransferData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'BluetoothTransferData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothReadData({required CoreOperitPluginSdkJsSdkResultsBluetoothReadData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'BluetoothReadData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothBleServicesData({required CoreOperitPluginSdkJsSdkResultsBluetoothBleServicesData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'BluetoothBleServicesData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothBleNotificationData({required CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'BluetoothBleNotificationData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.findFilesResultData({required FindFilesResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'FindFilesResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.grepResultData({required GrepResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'GrepResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.memoryLinkResultData({required MemoryLinkResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'MemoryLinkResultData', value: value);
  factory CoreOperitPluginSdkJsSdkResultsToolResultData.memoryLinkQueryResultData({required MemoryLinkQueryResultData value, }) => CoreOperitPluginSdkJsSdkResultsToolResultData._(tag: 'MemoryLinkQueryResultData', value: value);
  final String tag;
  final Object? value;

  factory CoreOperitPluginSdkJsSdkResultsToolResultData.fromJson(Object? json) {
    final map = json as Map<String, Object?>;
    final tag = map.keys.first;
    final data = map[tag] as Map<String, Object?>? ?? <String, Object?>{};
    return switch (tag) {
      'BooleanResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.booleanResultData(value: BooleanResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'StringResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.stringResultData(value: StringResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'SleepResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.sleepResultData(value: SleepResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'EnvironmentVariableReadResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.environmentVariableReadResultData(value: EnvironmentVariableReadResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'EnvironmentVariableWriteResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.environmentVariableWriteResultData(value: EnvironmentVariableWriteResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'IntResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.intResultData(value: IntResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'BinaryResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.binaryResultData(value: BinaryResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'FilePartContentData' => CoreOperitPluginSdkJsSdkResultsToolResultData.filePartContentData(value: FilePartContentData.fromJson(data['value'] as Map<String, Object?>), ),
      'DirectoryListingData' => CoreOperitPluginSdkJsSdkResultsToolResultData.directoryListingData(value: DirectoryListingData.fromJson(data['value'] as Map<String, Object?>), ),
      'FileContentData' => CoreOperitPluginSdkJsSdkResultsToolResultData.fileContentData(value: FileContentData.fromJson(data['value'] as Map<String, Object?>), ),
      'BinaryFileContentData' => CoreOperitPluginSdkJsSdkResultsToolResultData.binaryFileContentData(value: BinaryFileContentData.fromJson(data['value'] as Map<String, Object?>), ),
      'FileExistsData' => CoreOperitPluginSdkJsSdkResultsToolResultData.fileExistsData(value: FileExistsData.fromJson(data['value'] as Map<String, Object?>), ),
      'FileInfoData' => CoreOperitPluginSdkJsSdkResultsToolResultData.fileInfoData(value: FileInfoData.fromJson(data['value'] as Map<String, Object?>), ),
      'FileOperationData' => CoreOperitPluginSdkJsSdkResultsToolResultData.fileOperationData(value: FileOperationData.fromJson(data['value'] as Map<String, Object?>), ),
      'FileApplyResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.fileApplyResultData(value: FileApplyResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'HttpResponseData' => CoreOperitPluginSdkJsSdkResultsToolResultData.httpResponseData(value: HttpResponseData.fromJson(data['value'] as Map<String, Object?>), ),
      'HttpStreamEventData' => CoreOperitPluginSdkJsSdkResultsToolResultData.httpStreamEventData(value: HttpStreamEventData.fromJson(data['value'] as Map<String, Object?>), ),
      'SystemSettingData' => CoreOperitPluginSdkJsSdkResultsToolResultData.systemSettingData(value: SystemSettingData.fromJson(data['value'] as Map<String, Object?>), ),
      'AppOperationData' => CoreOperitPluginSdkJsSdkResultsToolResultData.appOperationData(value: AppOperationData.fromJson(data['value'] as Map<String, Object?>), ),
      'AppListData' => CoreOperitPluginSdkJsSdkResultsToolResultData.appListData(value: AppListData.fromJson(data['value'] as Map<String, Object?>), ),
      'AppUsageTimeResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.appUsageTimeResultData(value: AppUsageTimeResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'NotificationData' => CoreOperitPluginSdkJsSdkResultsToolResultData.notificationData(value: NotificationData.fromJson(data['value'] as Map<String, Object?>), ),
      'LocationData' => CoreOperitPluginSdkJsSdkResultsToolResultData.locationData(value: LocationData.fromJson(data['value'] as Map<String, Object?>), ),
      'DeviceInfoResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.deviceInfoResultData(value: DeviceInfoResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'MemoryQueryResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.memoryQueryResultData(value: MemoryQueryResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'ChatServiceStartResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.chatServiceStartResultData(value: ChatServiceStartResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'ChatCreationResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.chatCreationResultData(value: ChatCreationResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'ChatListResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.chatListResultData(value: ChatListResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'ChatFindResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.chatFindResultData(value: ChatFindResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'AgentStatusResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.agentStatusResultData(value: AgentStatusResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'ChatSwitchResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.chatSwitchResultData(value: ChatSwitchResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'ChatTitleUpdateResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.chatTitleUpdateResultData(value: ChatTitleUpdateResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'ChatDeleteResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.chatDeleteResultData(value: ChatDeleteResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'MessageSendResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.messageSendResultData(value: MessageSendResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'ChatMessagesResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.chatMessagesResultData(value: ChatMessagesResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'CharacterCardListResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.characterCardListResultData(value: CharacterCardListResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'VisitWebResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.visitWebResultData(value: VisitWebResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'TerminalInfoResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.terminalInfoResultData(value: TerminalInfoResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'TerminalCommandResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.terminalCommandResultData(value: TerminalCommandResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'TerminalStreamEventData' => CoreOperitPluginSdkJsSdkResultsToolResultData.terminalStreamEventData(value: TerminalStreamEventData.fromJson(data['value'] as Map<String, Object?>), ),
      'HiddenTerminalCommandResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.hiddenTerminalCommandResultData(value: HiddenTerminalCommandResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'TerminalSessionCreationResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.terminalSessionCreationResultData(value: TerminalSessionCreationResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'TerminalSessionCloseResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.terminalSessionCloseResultData(value: TerminalSessionCloseResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'TerminalSessionScreenResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.terminalSessionScreenResultData(value: TerminalSessionScreenResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'MusicPlaybackResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.musicPlaybackResultData(value: MusicPlaybackResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'BluetoothStateData' => CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothStateData(value: CoreOperitPluginSdkJsSdkResultsBluetoothStateData.fromJson(data['value'] as Map<String, Object?>), ),
      'BluetoothBondedDevicesData' => CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothBondedDevicesData(value: CoreOperitPluginSdkJsSdkResultsBluetoothBondedDevicesData.fromJson(data['value'] as Map<String, Object?>), ),
      'BluetoothScanResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothScanResultData(value: CoreOperitPluginSdkJsSdkResultsBluetoothScanResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'BluetoothSessionData' => CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothSessionData(value: CoreOperitPluginSdkJsSdkResultsBluetoothSessionData.fromJson(data['value'] as Map<String, Object?>), ),
      'BluetoothTransferData' => CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothTransferData(value: CoreOperitPluginSdkJsSdkResultsBluetoothTransferData.fromJson(data['value'] as Map<String, Object?>), ),
      'BluetoothReadData' => CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothReadData(value: CoreOperitPluginSdkJsSdkResultsBluetoothReadData.fromJson(data['value'] as Map<String, Object?>), ),
      'BluetoothBleServicesData' => CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothBleServicesData(value: CoreOperitPluginSdkJsSdkResultsBluetoothBleServicesData.fromJson(data['value'] as Map<String, Object?>), ),
      'BluetoothBleNotificationData' => CoreOperitPluginSdkJsSdkResultsToolResultData.bluetoothBleNotificationData(value: CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationData.fromJson(data['value'] as Map<String, Object?>), ),
      'FindFilesResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.findFilesResultData(value: FindFilesResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'GrepResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.grepResultData(value: GrepResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'MemoryLinkResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.memoryLinkResultData(value: MemoryLinkResultData.fromJson(data['value'] as Map<String, Object?>), ),
      'MemoryLinkQueryResultData' => CoreOperitPluginSdkJsSdkResultsToolResultData.memoryLinkQueryResultData(value: MemoryLinkQueryResultData.fromJson(data['value'] as Map<String, Object?>), ),
      _ => throw ArgumentError('Unknown CoreOperitPluginSdkJsSdkResultsToolResultData tag: $tag'),
    };
  }

  Map<String, Object?> toJson() {
    final data = <String, Object?>{
      if (tag == 'BooleanResultData') ...<String, Object?>{
        'value': (value as BooleanResultData).toJson(),
      },
      if (tag == 'StringResultData') ...<String, Object?>{
        'value': (value as StringResultData).toJson(),
      },
      if (tag == 'SleepResultData') ...<String, Object?>{
        'value': (value as SleepResultData).toJson(),
      },
      if (tag == 'EnvironmentVariableReadResultData') ...<String, Object?>{
        'value': (value as EnvironmentVariableReadResultData).toJson(),
      },
      if (tag == 'EnvironmentVariableWriteResultData') ...<String, Object?>{
        'value': (value as EnvironmentVariableWriteResultData).toJson(),
      },
      if (tag == 'IntResultData') ...<String, Object?>{
        'value': (value as IntResultData).toJson(),
      },
      if (tag == 'BinaryResultData') ...<String, Object?>{
        'value': (value as BinaryResultData).toJson(),
      },
      if (tag == 'FilePartContentData') ...<String, Object?>{
        'value': (value as FilePartContentData).toJson(),
      },
      if (tag == 'DirectoryListingData') ...<String, Object?>{
        'value': (value as DirectoryListingData).toJson(),
      },
      if (tag == 'FileContentData') ...<String, Object?>{
        'value': (value as FileContentData).toJson(),
      },
      if (tag == 'BinaryFileContentData') ...<String, Object?>{
        'value': (value as BinaryFileContentData).toJson(),
      },
      if (tag == 'FileExistsData') ...<String, Object?>{
        'value': (value as FileExistsData).toJson(),
      },
      if (tag == 'FileInfoData') ...<String, Object?>{
        'value': (value as FileInfoData).toJson(),
      },
      if (tag == 'FileOperationData') ...<String, Object?>{
        'value': (value as FileOperationData).toJson(),
      },
      if (tag == 'FileApplyResultData') ...<String, Object?>{
        'value': (value as FileApplyResultData).toJson(),
      },
      if (tag == 'HttpResponseData') ...<String, Object?>{
        'value': (value as HttpResponseData).toJson(),
      },
      if (tag == 'HttpStreamEventData') ...<String, Object?>{
        'value': (value as HttpStreamEventData).toJson(),
      },
      if (tag == 'SystemSettingData') ...<String, Object?>{
        'value': (value as SystemSettingData).toJson(),
      },
      if (tag == 'AppOperationData') ...<String, Object?>{
        'value': (value as AppOperationData).toJson(),
      },
      if (tag == 'AppListData') ...<String, Object?>{
        'value': (value as AppListData).toJson(),
      },
      if (tag == 'AppUsageTimeResultData') ...<String, Object?>{
        'value': (value as AppUsageTimeResultData).toJson(),
      },
      if (tag == 'NotificationData') ...<String, Object?>{
        'value': (value as NotificationData).toJson(),
      },
      if (tag == 'LocationData') ...<String, Object?>{
        'value': (value as LocationData).toJson(),
      },
      if (tag == 'DeviceInfoResultData') ...<String, Object?>{
        'value': (value as DeviceInfoResultData).toJson(),
      },
      if (tag == 'MemoryQueryResultData') ...<String, Object?>{
        'value': (value as MemoryQueryResultData).toJson(),
      },
      if (tag == 'ChatServiceStartResultData') ...<String, Object?>{
        'value': (value as ChatServiceStartResultData).toJson(),
      },
      if (tag == 'ChatCreationResultData') ...<String, Object?>{
        'value': (value as ChatCreationResultData).toJson(),
      },
      if (tag == 'ChatListResultData') ...<String, Object?>{
        'value': (value as ChatListResultData).toJson(),
      },
      if (tag == 'ChatFindResultData') ...<String, Object?>{
        'value': (value as ChatFindResultData).toJson(),
      },
      if (tag == 'AgentStatusResultData') ...<String, Object?>{
        'value': (value as AgentStatusResultData).toJson(),
      },
      if (tag == 'ChatSwitchResultData') ...<String, Object?>{
        'value': (value as ChatSwitchResultData).toJson(),
      },
      if (tag == 'ChatTitleUpdateResultData') ...<String, Object?>{
        'value': (value as ChatTitleUpdateResultData).toJson(),
      },
      if (tag == 'ChatDeleteResultData') ...<String, Object?>{
        'value': (value as ChatDeleteResultData).toJson(),
      },
      if (tag == 'MessageSendResultData') ...<String, Object?>{
        'value': (value as MessageSendResultData).toJson(),
      },
      if (tag == 'ChatMessagesResultData') ...<String, Object?>{
        'value': (value as ChatMessagesResultData).toJson(),
      },
      if (tag == 'CharacterCardListResultData') ...<String, Object?>{
        'value': (value as CharacterCardListResultData).toJson(),
      },
      if (tag == 'VisitWebResultData') ...<String, Object?>{
        'value': (value as VisitWebResultData).toJson(),
      },
      if (tag == 'TerminalInfoResultData') ...<String, Object?>{
        'value': (value as TerminalInfoResultData).toJson(),
      },
      if (tag == 'TerminalCommandResultData') ...<String, Object?>{
        'value': (value as TerminalCommandResultData).toJson(),
      },
      if (tag == 'TerminalStreamEventData') ...<String, Object?>{
        'value': (value as TerminalStreamEventData).toJson(),
      },
      if (tag == 'HiddenTerminalCommandResultData') ...<String, Object?>{
        'value': (value as HiddenTerminalCommandResultData).toJson(),
      },
      if (tag == 'TerminalSessionCreationResultData') ...<String, Object?>{
        'value': (value as TerminalSessionCreationResultData).toJson(),
      },
      if (tag == 'TerminalSessionCloseResultData') ...<String, Object?>{
        'value': (value as TerminalSessionCloseResultData).toJson(),
      },
      if (tag == 'TerminalSessionScreenResultData') ...<String, Object?>{
        'value': (value as TerminalSessionScreenResultData).toJson(),
      },
      if (tag == 'MusicPlaybackResultData') ...<String, Object?>{
        'value': (value as MusicPlaybackResultData).toJson(),
      },
      if (tag == 'BluetoothStateData') ...<String, Object?>{
        'value': (value as CoreOperitPluginSdkJsSdkResultsBluetoothStateData).toJson(),
      },
      if (tag == 'BluetoothBondedDevicesData') ...<String, Object?>{
        'value': (value as CoreOperitPluginSdkJsSdkResultsBluetoothBondedDevicesData).toJson(),
      },
      if (tag == 'BluetoothScanResultData') ...<String, Object?>{
        'value': (value as CoreOperitPluginSdkJsSdkResultsBluetoothScanResultData).toJson(),
      },
      if (tag == 'BluetoothSessionData') ...<String, Object?>{
        'value': (value as CoreOperitPluginSdkJsSdkResultsBluetoothSessionData).toJson(),
      },
      if (tag == 'BluetoothTransferData') ...<String, Object?>{
        'value': (value as CoreOperitPluginSdkJsSdkResultsBluetoothTransferData).toJson(),
      },
      if (tag == 'BluetoothReadData') ...<String, Object?>{
        'value': (value as CoreOperitPluginSdkJsSdkResultsBluetoothReadData).toJson(),
      },
      if (tag == 'BluetoothBleServicesData') ...<String, Object?>{
        'value': (value as CoreOperitPluginSdkJsSdkResultsBluetoothBleServicesData).toJson(),
      },
      if (tag == 'BluetoothBleNotificationData') ...<String, Object?>{
        'value': (value as CoreOperitPluginSdkJsSdkResultsBluetoothBleNotificationData).toJson(),
      },
      if (tag == 'FindFilesResultData') ...<String, Object?>{
        'value': (value as FindFilesResultData).toJson(),
      },
      if (tag == 'GrepResultData') ...<String, Object?>{
        'value': (value as GrepResultData).toJson(),
      },
      if (tag == 'MemoryLinkResultData') ...<String, Object?>{
        'value': (value as MemoryLinkResultData).toJson(),
      },
      if (tag == 'MemoryLinkQueryResultData') ...<String, Object?>{
        'value': (value as MemoryLinkQueryResultData).toJson(),
      },
    };
    return <String, Object?>{tag: data};
  }
}

/// Generated Dart model for Rust type `operit_plugin_sdk::js_sdk::results::VisitWebResultData`.
class VisitWebResultData {
  const VisitWebResultData({
    required this.url,
    required this.title,
    required this.content,
    required this.metadata,
    required this.links,
    required this.imageLinks,
    required this.visitKey,
    required this.contentSavedTo,
    required this.contentTruncated,
    required this.originalContentLength,
  });

  factory VisitWebResultData.fromJson(Map<String, Object?> json) {
    return VisitWebResultData(
      url: json['url'] as String,
      title: json['title'] as String,
      content: json['content'] as String,
      metadata: (json['metadata'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
      links: (json['links'] as List<Object?>).map((item) => LinkData.fromJson(item as Map<String, Object?>)).toList(growable: false),
      imageLinks: (json['imageLinks'] as List<Object?>).map((item) => item as String).toList(growable: false),
      visitKey: json['visitKey'] == null ? null : json['visitKey'] as String,
      contentSavedTo: json['contentSavedTo'] == null ? null : json['contentSavedTo'] as String,
      contentTruncated: json['contentTruncated'] as bool,
      originalContentLength: json['originalContentLength'] == null ? null : json['originalContentLength'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'url': url,
      'title': title,
      'content': content,
      'metadata': metadata.map((key, value) => MapEntry(key, value)),
      'links': links.map((item) => item.toJson()).toList(growable: false),
      'imageLinks': imageLinks.map((item) => item).toList(growable: false),
      'visitKey': visitKey,
      'contentSavedTo': contentSavedTo,
      'contentTruncated': contentTruncated,
      'originalContentLength': originalContentLength,
    };
  }

  /// Rust field `url` serialized as `url`.
  final String url;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `content` serialized as `content`.
  final String content;
  /// Rust field `metadata` serialized as `metadata`.
  final Map<String, String> metadata;
  /// Rust field `links` serialized as `links`.
  final List<LinkData> links;
  /// Rust field `imageLinks` serialized as `imageLinks`.
  final List<String> imageLinks;
  /// Rust field `visitKey` serialized as `visitKey`.
  final String? visitKey;
  /// Rust field `contentSavedTo` serialized as `contentSavedTo`.
  final String? contentSavedTo;
  /// Rust field `contentTruncated` serialized as `contentTruncated`.
  final bool contentTruncated;
  /// Rust field `originalContentLength` serialized as `originalContentLength`.
  final int? originalContentLength;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::package::EnvVar`.
class EnvVar {
  const EnvVar({
    required this.name,
    required this.description,
    required this.requiredValue,
    required this.defaultValue,
  });

  factory EnvVar.fromJson(Map<String, Object?> json) {
    return EnvVar(
      name: json['name'] as String,
      description: LocalizedText.fromJson(json['description'] as Map<String, Object?>),
      requiredValue: json['required'] as bool,
      defaultValue: json['default_value'] == null ? null : json['default_value'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'description': description.toJson(),
      'required': requiredValue,
      'default_value': defaultValue,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final LocalizedText description;
  /// Rust field `required` serialized as `required`.
  final bool requiredValue;
  /// Rust field `default_value` serialized as `default_value`.
  final String? defaultValue;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::package::LocalizedText`.
class LocalizedText {
  const LocalizedText({
    required this.values,
  });

  factory LocalizedText.fromJson(Map<String, Object?> json) {
    return LocalizedText(
      values: (json['values'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'values': values.map((key, value) => MapEntry(key, value)),
    };
  }

  /// Rust field `values` serialized as `values`.
  final Map<String, String> values;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::package::PackageTool`.
class PackageTool {
  const PackageTool({
    required this.name,
    required this.description,
    required this.parameters,
    required this.script,
    required this.advice,
  });

  factory PackageTool.fromJson(Map<String, Object?> json) {
    return PackageTool(
      name: json['name'] as String,
      description: LocalizedText.fromJson(json['description'] as Map<String, Object?>),
      parameters: (json['parameters'] as List<Object?>).map((item) => PackageToolParameter.fromJson(item as Map<String, Object?>)).toList(growable: false),
      script: json['script'] as String,
      advice: json['advice'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'description': description.toJson(),
      'parameters': parameters.map((item) => item.toJson()).toList(growable: false),
      'script': script,
      'advice': advice,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final LocalizedText description;
  /// Rust field `parameters` serialized as `parameters`.
  final List<PackageToolParameter> parameters;
  /// Rust field `script` serialized as `script`.
  final String script;
  /// Rust field `advice` serialized as `advice`.
  final bool advice;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::package::PackageToolParameter`.
class PackageToolParameter {
  const PackageToolParameter({
    required this.name,
    required this.description,
    required this.parameterType,
    required this.requiredValue,
  });

  factory PackageToolParameter.fromJson(Map<String, Object?> json) {
    return PackageToolParameter(
      name: json['name'] as String,
      description: LocalizedText.fromJson(json['description'] as Map<String, Object?>),
      parameterType: json['parameter_type'] as String,
      requiredValue: json['required'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'description': description.toJson(),
      'parameter_type': parameterType,
      'required': requiredValue,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final LocalizedText description;
  /// Rust field `parameter_type` serialized as `parameter_type`.
  final String parameterType;
  /// Rust field `required` serialized as `required`.
  final bool requiredValue;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::package::PublishablePackageSource`.
class PublishablePackageSource {
  const PublishablePackageSource({
    required this.packageName,
    required this.displayName,
    required this.description,
    required this.author,
    required this.sourcePath,
    required this.sourceFileName,
    required this.fileExtension,
    required this.isToolPkg,
    required this.inferredVersion,
  });

  factory PublishablePackageSource.fromJson(Map<String, Object?> json) {
    return PublishablePackageSource(
      packageName: json['packageName'] as String,
      displayName: json['displayName'] as String,
      description: json['description'] as String,
      author: (json['author'] as List<Object?>).map((item) => item as String).toList(growable: false),
      sourcePath: json['sourcePath'] as String,
      sourceFileName: json['sourceFileName'] as String,
      fileExtension: json['fileExtension'] as String,
      isToolPkg: json['isToolPkg'] as bool,
      inferredVersion: json['inferredVersion'] == null ? null : json['inferredVersion'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'packageName': packageName,
      'displayName': displayName,
      'description': description,
      'author': author.map((item) => item).toList(growable: false),
      'sourcePath': sourcePath,
      'sourceFileName': sourceFileName,
      'fileExtension': fileExtension,
      'isToolPkg': isToolPkg,
      'inferredVersion': inferredVersion,
    };
  }

  /// Rust field `packageName` serialized as `packageName`.
  final String packageName;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `author` serialized as `author`.
  final List<String> author;
  /// Rust field `sourcePath` serialized as `sourcePath`.
  final String sourcePath;
  /// Rust field `sourceFileName` serialized as `sourceFileName`.
  final String sourceFileName;
  /// Rust field `fileExtension` serialized as `fileExtension`.
  final String fileExtension;
  /// Rust field `isToolPkg` serialized as `isToolPkg`.
  final bool isToolPkg;
  /// Rust field `inferredVersion` serialized as `inferredVersion`.
  final String? inferredVersion;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::package::ToolPackage`.
class ToolPackage {
  const ToolPackage({
    required this.name,
    required this.description,
    required this.tools,
    required this.states,
    required this.env,
    required this.isBuiltIn,
    required this.enabledByDefault,
    required this.displayName,
    required this.category,
    required this.author,
  });

  factory ToolPackage.fromJson(Map<String, Object?> json) {
    return ToolPackage(
      name: json['name'] as String,
      description: LocalizedText.fromJson(json['description'] as Map<String, Object?>),
      tools: (json['tools'] as List<Object?>).map((item) => PackageTool.fromJson(item as Map<String, Object?>)).toList(growable: false),
      states: (json['states'] as List<Object?>).map((item) => ToolPackageState.fromJson(item as Map<String, Object?>)).toList(growable: false),
      env: (json['env'] as List<Object?>).map((item) => EnvVar.fromJson(item as Map<String, Object?>)).toList(growable: false),
      isBuiltIn: json['is_built_in'] as bool,
      enabledByDefault: json['enabled_by_default'] as bool,
      displayName: LocalizedText.fromJson(json['display_name'] as Map<String, Object?>),
      category: json['category'] as String,
      author: (json['author'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'description': description.toJson(),
      'tools': tools.map((item) => item.toJson()).toList(growable: false),
      'states': states.map((item) => item.toJson()).toList(growable: false),
      'env': env.map((item) => item.toJson()).toList(growable: false),
      'is_built_in': isBuiltIn,
      'enabled_by_default': enabledByDefault,
      'display_name': displayName.toJson(),
      'category': category,
      'author': author.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final LocalizedText description;
  /// Rust field `tools` serialized as `tools`.
  final List<PackageTool> tools;
  /// Rust field `states` serialized as `states`.
  final List<ToolPackageState> states;
  /// Rust field `env` serialized as `env`.
  final List<EnvVar> env;
  /// Rust field `is_built_in` serialized as `is_built_in`.
  final bool isBuiltIn;
  /// Rust field `enabled_by_default` serialized as `enabled_by_default`.
  final bool enabledByDefault;
  /// Rust field `display_name` serialized as `display_name`.
  final LocalizedText displayName;
  /// Rust field `category` serialized as `category`.
  final String category;
  /// Rust field `author` serialized as `author`.
  final List<String> author;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::package::ToolPackageState`.
class ToolPackageState {
  const ToolPackageState({
    required this.id,
    required this.condition,
    required this.inheritTools,
    required this.excludeTools,
    required this.tools,
  });

  factory ToolPackageState.fromJson(Map<String, Object?> json) {
    return ToolPackageState(
      id: json['id'] as String,
      condition: json['condition'] as String,
      inheritTools: json['inherit_tools'] as bool,
      excludeTools: (json['exclude_tools'] as List<Object?>).map((item) => item as String).toList(growable: false),
      tools: (json['tools'] as List<Object?>).map((item) => PackageTool.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'condition': condition,
      'inherit_tools': inheritTools,
      'exclude_tools': excludeTools.map((item) => item).toList(growable: false),
      'tools': tools.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `condition` serialized as `condition`.
  final String condition;
  /// Rust field `inherit_tools` serialized as `inherit_tools`.
  final bool inheritTools;
  /// Rust field `exclude_tools` serialized as `exclude_tools`.
  final List<String> excludeTools;
  /// Rust field `tools` serialized as `tools`.
  final List<PackageTool> tools;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgContainerDetails`.
class ToolPkgContainerDetails {
  const ToolPkgContainerDetails({
    required this.packageName,
    required this.displayName,
    required this.description,
    required this.version,
    required this.author,
    required this.resourceCount,
    required this.workspaceTemplateCount,
    required this.uiModuleCount,
    required this.toolboxUiModules,
    required this.subpackages,
    required this.workspaceTemplates,
  });

  factory ToolPkgContainerDetails.fromJson(Map<String, Object?> json) {
    return ToolPkgContainerDetails(
      packageName: json['packageName'] as String,
      displayName: json['displayName'] as String,
      description: json['description'] as String,
      version: json['version'] as String,
      author: (json['author'] as List<Object?>).map((item) => item as String).toList(growable: false),
      resourceCount: json['resourceCount'] as int,
      workspaceTemplateCount: json['workspaceTemplateCount'] as int,
      uiModuleCount: json['uiModuleCount'] as int,
      toolboxUiModules: (json['toolboxUiModules'] as List<Object?>).map((item) => ToolPkgToolboxUiModule.fromJson(item as Map<String, Object?>)).toList(growable: false),
      subpackages: (json['subpackages'] as List<Object?>).map((item) => ToolPkgSubpackageInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
      workspaceTemplates: (json['workspaceTemplates'] as List<Object?>).map((item) => ToolPkgWorkspaceTemplate.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'packageName': packageName,
      'displayName': displayName,
      'description': description,
      'version': version,
      'author': author.map((item) => item).toList(growable: false),
      'resourceCount': resourceCount,
      'workspaceTemplateCount': workspaceTemplateCount,
      'uiModuleCount': uiModuleCount,
      'toolboxUiModules': toolboxUiModules.map((item) => item.toJson()).toList(growable: false),
      'subpackages': subpackages.map((item) => item.toJson()).toList(growable: false),
      'workspaceTemplates': workspaceTemplates.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `packageName` serialized as `packageName`.
  final String packageName;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `version` serialized as `version`.
  final String version;
  /// Rust field `author` serialized as `author`.
  final List<String> author;
  /// Rust field `resourceCount` serialized as `resourceCount`.
  final int resourceCount;
  /// Rust field `workspaceTemplateCount` serialized as `workspaceTemplateCount`.
  final int workspaceTemplateCount;
  /// Rust field `uiModuleCount` serialized as `uiModuleCount`.
  final int uiModuleCount;
  /// Rust field `toolboxUiModules` serialized as `toolboxUiModules`.
  final List<ToolPkgToolboxUiModule> toolboxUiModules;
  /// Rust field `subpackages` serialized as `subpackages`.
  final List<ToolPkgSubpackageInfo> subpackages;
  /// Rust field `workspaceTemplates` serialized as `workspaceTemplates`.
  final List<ToolPkgWorkspaceTemplate> workspaceTemplates;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgDesktopWidget`.
class ToolPkgDesktopWidget {
  const ToolPkgDesktopWidget({
    required this.containerPackageName,
    required this.toolPkgId,
    required this.widgetId,
    required this.routeId,
    required this.renderRouteId,
    required this.title,
    required this.subtitle,
    required this.description,
    required this.icon,
    required this.order,
  });

  factory ToolPkgDesktopWidget.fromJson(Map<String, Object?> json) {
    return ToolPkgDesktopWidget(
      containerPackageName: json['containerPackageName'] as String,
      toolPkgId: json['toolPkgId'] as String,
      widgetId: json['widgetId'] as String,
      routeId: json['routeId'] as String,
      renderRouteId: json['renderRouteId'] as String,
      title: json['title'] as String,
      subtitle: json['subtitle'] as String,
      description: json['description'] as String,
      icon: json['icon'] == null ? null : json['icon'] as String,
      order: json['order'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'containerPackageName': containerPackageName,
      'toolPkgId': toolPkgId,
      'widgetId': widgetId,
      'routeId': routeId,
      'renderRouteId': renderRouteId,
      'title': title,
      'subtitle': subtitle,
      'description': description,
      'icon': icon,
      'order': order,
    };
  }

  /// Rust field `containerPackageName` serialized as `containerPackageName`.
  final String containerPackageName;
  /// Rust field `toolPkgId` serialized as `toolPkgId`.
  final String toolPkgId;
  /// Rust field `widgetId` serialized as `widgetId`.
  final String widgetId;
  /// Rust field `routeId` serialized as `routeId`.
  final String routeId;
  /// Rust field `renderRouteId` serialized as `renderRouteId`.
  final String renderRouteId;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `subtitle` serialized as `subtitle`.
  final String subtitle;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `icon` serialized as `icon`.
  final String? icon;
  /// Rust field `order` serialized as `order`.
  final int order;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgNavigationActionHook`.
class ToolPkgNavigationActionHook {
  const ToolPkgNavigationActionHook({
    required this.functionName,
    required this.functionSource,
  });

  factory ToolPkgNavigationActionHook.fromJson(Map<String, Object?> json) {
    return ToolPkgNavigationActionHook(
      functionName: json['functionName'] as String,
      functionSource: json['functionSource'] == null ? null : json['functionSource'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'functionName': functionName,
      'functionSource': functionSource,
    };
  }

  /// Rust field `functionName` serialized as `functionName`.
  final String functionName;
  /// Rust field `functionSource` serialized as `functionSource`.
  final String? functionSource;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgNavigationEntry`.
class ToolPkgNavigationEntry {
  const ToolPkgNavigationEntry({
    required this.containerPackageName,
    required this.toolPkgId,
    required this.entryId,
    required this.routeId,
    required this.surface,
    required this.title,
    required this.description,
    required this.action,
    required this.icon,
    required this.order,
  });

  factory ToolPkgNavigationEntry.fromJson(Map<String, Object?> json) {
    return ToolPkgNavigationEntry(
      containerPackageName: json['containerPackageName'] as String,
      toolPkgId: json['toolPkgId'] as String,
      entryId: json['entryId'] as String,
      routeId: json['routeId'] as String,
      surface: json['surface'] as String,
      title: json['title'] as String,
      description: json['description'] as String,
      action: json['action'] == null ? null : ToolPkgNavigationActionHook.fromJson(json['action'] as Map<String, Object?>),
      icon: json['icon'] == null ? null : json['icon'] as String,
      order: json['order'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'containerPackageName': containerPackageName,
      'toolPkgId': toolPkgId,
      'entryId': entryId,
      'routeId': routeId,
      'surface': surface,
      'title': title,
      'description': description,
      'action': action?.toJson(),
      'icon': icon,
      'order': order,
    };
  }

  /// Rust field `containerPackageName` serialized as `containerPackageName`.
  final String containerPackageName;
  /// Rust field `toolPkgId` serialized as `toolPkgId`.
  final String toolPkgId;
  /// Rust field `entryId` serialized as `entryId`.
  final String entryId;
  /// Rust field `routeId` serialized as `routeId`.
  final String routeId;
  /// Rust field `surface` serialized as `surface`.
  final String surface;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `action` serialized as `action`.
  final ToolPkgNavigationActionHook? action;
  /// Rust field `icon` serialized as `icon`.
  final String? icon;
  /// Rust field `order` serialized as `order`.
  final int order;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgSubpackageInfo`.
class ToolPkgSubpackageInfo {
  const ToolPkgSubpackageInfo({
    required this.packageName,
    required this.subpackageId,
    required this.displayName,
    required this.description,
    required this.enabledByDefault,
    required this.toolCount,
    required this.enabled,
  });

  factory ToolPkgSubpackageInfo.fromJson(Map<String, Object?> json) {
    return ToolPkgSubpackageInfo(
      packageName: json['packageName'] as String,
      subpackageId: json['subpackageId'] as String,
      displayName: json['displayName'] as String,
      description: json['description'] as String,
      enabledByDefault: json['enabledByDefault'] as bool,
      toolCount: json['toolCount'] as int,
      enabled: json['enabled'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'packageName': packageName,
      'subpackageId': subpackageId,
      'displayName': displayName,
      'description': description,
      'enabledByDefault': enabledByDefault,
      'toolCount': toolCount,
      'enabled': enabled,
    };
  }

  /// Rust field `packageName` serialized as `packageName`.
  final String packageName;
  /// Rust field `subpackageId` serialized as `subpackageId`.
  final String subpackageId;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `enabledByDefault` serialized as `enabledByDefault`.
  final bool enabledByDefault;
  /// Rust field `toolCount` serialized as `toolCount`.
  final int toolCount;
  /// Rust field `enabled` serialized as `enabled`.
  final bool enabled;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgToolboxUiModule`.
class ToolPkgToolboxUiModule {
  const ToolPkgToolboxUiModule({
    required this.containerPackageName,
    required this.toolPkgId,
    required this.routeId,
    required this.uiModuleId,
    required this.runtime,
    required this.screen,
    required this.title,
    required this.description,
    required this.moduleSpec,
    required this.keepAlive,
  });

  factory ToolPkgToolboxUiModule.fromJson(Map<String, Object?> json) {
    return ToolPkgToolboxUiModule(
      containerPackageName: json['containerPackageName'] as String,
      toolPkgId: json['toolPkgId'] as String,
      routeId: json['routeId'] as String,
      uiModuleId: json['uiModuleId'] as String,
      runtime: json['runtime'] as String,
      screen: json['screen'] as String,
      title: json['title'] as String,
      description: json['description'] as String,
      moduleSpec: (json['moduleSpec'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value)),
      keepAlive: json['keepAlive'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'containerPackageName': containerPackageName,
      'toolPkgId': toolPkgId,
      'routeId': routeId,
      'uiModuleId': uiModuleId,
      'runtime': runtime,
      'screen': screen,
      'title': title,
      'description': description,
      'moduleSpec': moduleSpec.map((key, value) => MapEntry(key, value)),
      'keepAlive': keepAlive,
    };
  }

  /// Rust field `containerPackageName` serialized as `containerPackageName`.
  final String containerPackageName;
  /// Rust field `toolPkgId` serialized as `toolPkgId`.
  final String toolPkgId;
  /// Rust field `routeId` serialized as `routeId`.
  final String routeId;
  /// Rust field `uiModuleId` serialized as `uiModuleId`.
  final String uiModuleId;
  /// Rust field `runtime` serialized as `runtime`.
  final String runtime;
  /// Rust field `screen` serialized as `screen`.
  final String screen;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `moduleSpec` serialized as `moduleSpec`.
  final Map<String, Object?> moduleSpec;
  /// Rust field `keepAlive` serialized as `keepAlive`.
  final bool keepAlive;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgUiRoute`.
class ToolPkgUiRoute {
  const ToolPkgUiRoute({
    required this.containerPackageName,
    required this.toolPkgId,
    required this.routeId,
    required this.uiModuleId,
    required this.runtime,
    required this.screen,
    required this.title,
    required this.description,
    required this.moduleSpec,
    required this.keepAlive,
  });

  factory ToolPkgUiRoute.fromJson(Map<String, Object?> json) {
    return ToolPkgUiRoute(
      containerPackageName: json['containerPackageName'] as String,
      toolPkgId: json['toolPkgId'] as String,
      routeId: json['routeId'] as String,
      uiModuleId: json['uiModuleId'] as String,
      runtime: json['runtime'] as String,
      screen: json['screen'] as String,
      title: json['title'] as String,
      description: json['description'] as String,
      moduleSpec: (json['moduleSpec'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value)),
      keepAlive: json['keepAlive'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'containerPackageName': containerPackageName,
      'toolPkgId': toolPkgId,
      'routeId': routeId,
      'uiModuleId': uiModuleId,
      'runtime': runtime,
      'screen': screen,
      'title': title,
      'description': description,
      'moduleSpec': moduleSpec.map((key, value) => MapEntry(key, value)),
      'keepAlive': keepAlive,
    };
  }

  /// Rust field `containerPackageName` serialized as `containerPackageName`.
  final String containerPackageName;
  /// Rust field `toolPkgId` serialized as `toolPkgId`.
  final String toolPkgId;
  /// Rust field `routeId` serialized as `routeId`.
  final String routeId;
  /// Rust field `uiModuleId` serialized as `uiModuleId`.
  final String uiModuleId;
  /// Rust field `runtime` serialized as `runtime`.
  final String runtime;
  /// Rust field `screen` serialized as `screen`.
  final String screen;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `moduleSpec` serialized as `moduleSpec`.
  final Map<String, Object?> moduleSpec;
  /// Rust field `keepAlive` serialized as `keepAlive`.
  final bool keepAlive;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgWorkspaceTemplate`.
class ToolPkgWorkspaceTemplate {
  const ToolPkgWorkspaceTemplate({
    required this.containerPackageName,
    required this.toolPkgId,
    required this.templateId,
    required this.displayName,
    required this.description,
    required this.resourceKey,
    required this.projectType,
  });

  factory ToolPkgWorkspaceTemplate.fromJson(Map<String, Object?> json) {
    return ToolPkgWorkspaceTemplate(
      containerPackageName: json['containerPackageName'] as String,
      toolPkgId: json['toolPkgId'] as String,
      templateId: json['templateId'] as String,
      displayName: json['displayName'] as String,
      description: json['description'] as String,
      resourceKey: json['resourceKey'] as String,
      projectType: json['projectType'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'containerPackageName': containerPackageName,
      'toolPkgId': toolPkgId,
      'templateId': templateId,
      'displayName': displayName,
      'description': description,
      'resourceKey': resourceKey,
      'projectType': projectType,
    };
  }

  /// Rust field `containerPackageName` serialized as `containerPackageName`.
  final String containerPackageName;
  /// Rust field `toolPkgId` serialized as `toolPkgId`.
  final String toolPkgId;
  /// Rust field `templateId` serialized as `templateId`.
  final String templateId;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `resourceKey` serialized as `resourceKey`.
  final String resourceKey;
  /// Rust field `projectType` serialized as `projectType`.
  final String projectType;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgWorkspaceTemplateImportResult`.
class ToolPkgWorkspaceTemplateImportResult {
  const ToolPkgWorkspaceTemplateImportResult({
    required this.containerPackageName,
    required this.toolPkgId,
    required this.templateId,
    required this.workspacePath,
    required this.workspaceConfig,
  });

  factory ToolPkgWorkspaceTemplateImportResult.fromJson(Map<String, Object?> json) {
    return ToolPkgWorkspaceTemplateImportResult(
      containerPackageName: json['containerPackageName'] as String,
      toolPkgId: json['toolPkgId'] as String,
      templateId: json['templateId'] as String,
      workspacePath: json['workspacePath'] as String,
      workspaceConfig: json['workspaceConfig'],
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'containerPackageName': containerPackageName,
      'toolPkgId': toolPkgId,
      'templateId': templateId,
      'workspacePath': workspacePath,
      'workspaceConfig': workspaceConfig,
    };
  }

  /// Rust field `containerPackageName` serialized as `containerPackageName`.
  final String containerPackageName;
  /// Rust field `toolPkgId` serialized as `toolPkgId`.
  final String toolPkgId;
  /// Rust field `templateId` serialized as `templateId`.
  final String templateId;
  /// Rust field `workspacePath` serialized as `workspacePath`.
  final String workspacePath;
  /// Rust field `workspaceConfig` serialized as `workspaceConfig`.
  final Object? workspaceConfig;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAiProviderHandlerRuntime`.
class ToolPkgAiProviderHandlerRuntime {
  const ToolPkgAiProviderHandlerRuntime({
    required this.function,
    required this.functionSource,
  });

  factory ToolPkgAiProviderHandlerRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgAiProviderHandlerRuntime(
      function: json['function'] as String,
      functionSource: json['functionSource'] == null ? null : json['functionSource'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'function': function,
      'functionSource': functionSource,
    };
  }

  /// Rust field `function` serialized as `function`.
  final String function;
  /// Rust field `functionSource` serialized as `functionSource`.
  final String? functionSource;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAiProviderRuntime`.
class ToolPkgAiProviderRuntime {
  const ToolPkgAiProviderRuntime({
    required this.id,
    required this.displayName,
    required this.description,
    required this.listModelsHandler,
    required this.sendMessageHandler,
    required this.testConnectionHandler,
    required this.calculateInputTokensHandler,
  });

  factory ToolPkgAiProviderRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgAiProviderRuntime(
      id: json['id'] as String,
      displayName: json['displayName'] as String,
      description: json['description'] as String,
      listModelsHandler: ToolPkgAiProviderHandlerRuntime.fromJson(json['listModelsHandler'] as Map<String, Object?>),
      sendMessageHandler: ToolPkgAiProviderHandlerRuntime.fromJson(json['sendMessageHandler'] as Map<String, Object?>),
      testConnectionHandler: ToolPkgAiProviderHandlerRuntime.fromJson(json['testConnectionHandler'] as Map<String, Object?>),
      calculateInputTokensHandler: ToolPkgAiProviderHandlerRuntime.fromJson(json['calculateInputTokensHandler'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'displayName': displayName,
      'description': description,
      'listModelsHandler': listModelsHandler.toJson(),
      'sendMessageHandler': sendMessageHandler.toJson(),
      'testConnectionHandler': testConnectionHandler.toJson(),
      'calculateInputTokensHandler': calculateInputTokensHandler.toJson(),
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `listModelsHandler` serialized as `listModelsHandler`.
  final ToolPkgAiProviderHandlerRuntime listModelsHandler;
  /// Rust field `sendMessageHandler` serialized as `sendMessageHandler`.
  final ToolPkgAiProviderHandlerRuntime sendMessageHandler;
  /// Rust field `testConnectionHandler` serialized as `testConnectionHandler`.
  final ToolPkgAiProviderHandlerRuntime testConnectionHandler;
  /// Rust field `calculateInputTokensHandler` serialized as `calculateInputTokensHandler`.
  final ToolPkgAiProviderHandlerRuntime calculateInputTokensHandler;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAppLifecycleHookRuntime`.
class ToolPkgAppLifecycleHookRuntime {
  const ToolPkgAppLifecycleHookRuntime({
    required this.id,
    required this.event,
    required this.function,
    required this.functionSource,
  });

  factory ToolPkgAppLifecycleHookRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgAppLifecycleHookRuntime(
      id: json['id'] as String,
      event: json['event'] as String,
      function: json['function'] as String,
      functionSource: json['functionSource'] == null ? null : json['functionSource'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'event': event,
      'function': function,
      'functionSource': functionSource,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `event` serialized as `event`.
  final String event;
  /// Rust field `function` serialized as `function`.
  final String function;
  /// Rust field `functionSource` serialized as `functionSource`.
  final String? functionSource;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime`.
class ToolPkgContainerRuntime {
  const ToolPkgContainerRuntime({
    required this.packageName,
    required this.displayName,
    required this.description,
    required this.version,
    required this.author,
    required this.mainEntry,
    required this.sourceType,
    required this.sourcePath,
    required this.subpackages,
    required this.resources,
    required this.wasmModules,
    required this.workflowTemplates,
    required this.workspaceTemplates,
    required this.uiModules,
    required this.uiRoutes,
    required this.navigationEntries,
    required this.desktopWidgets,
    required this.appLifecycleHooks,
    required this.messageProcessingPlugins,
    required this.xmlRenderPlugins,
    required this.inputMenuTogglePlugins,
    required this.chatInputHooks,
    required this.chatViewHooks,
    required this.chatMessageHooks,
    required this.hostEventHooks,
    required this.toolLifecycleHooks,
    required this.promptInputHooks,
    required this.promptHistoryHooks,
    required this.promptEstimateHistoryHooks,
    required this.systemPromptComposeHooks,
    required this.toolPromptComposeHooks,
    required this.promptFinalizeHooks,
    required this.promptEstimateFinalizeHooks,
    required this.summaryGenerateHooks,
    required this.aiProviders,
  });

  factory ToolPkgContainerRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgContainerRuntime(
      packageName: json['packageName'] as String,
      displayName: LocalizedText.fromJson(json['displayName'] as Map<String, Object?>),
      description: LocalizedText.fromJson(json['description'] as Map<String, Object?>),
      version: json['version'] as String,
      author: (json['author'] as List<Object?>).map((item) => item as String).toList(growable: false),
      mainEntry: json['mainEntry'] as String,
      sourceType: ToolPkgSourceType.fromJson(json['sourceType']),
      sourcePath: json['sourcePath'] as String,
      subpackages: (json['subpackages'] as List<Object?>).map((item) => ToolPkgSubpackageRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      resources: (json['resources'] as List<Object?>).map((item) => ToolPkgResourceRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      wasmModules: (json['wasmModules'] as List<Object?>).map((item) => ToolPkgWasmModuleRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      workflowTemplates: (json['workflowTemplates'] as List<Object?>).map((item) => ToolPkgWorkflowTemplateRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      workspaceTemplates: (json['workspaceTemplates'] as List<Object?>).map((item) => ToolPkgWorkspaceTemplateRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      uiModules: (json['uiModules'] as List<Object?>).map((item) => ToolPkgUiModuleRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      uiRoutes: (json['uiRoutes'] as List<Object?>).map((item) => ToolPkgUiRouteRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      navigationEntries: (json['navigationEntries'] as List<Object?>).map((item) => ToolPkgNavigationEntryRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      desktopWidgets: (json['desktopWidgets'] as List<Object?>).map((item) => ToolPkgDesktopWidgetRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      appLifecycleHooks: (json['appLifecycleHooks'] as List<Object?>).map((item) => ToolPkgAppLifecycleHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      messageProcessingPlugins: (json['messageProcessingPlugins'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      xmlRenderPlugins: (json['xmlRenderPlugins'] as List<Object?>).map((item) => ToolPkgTagFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      inputMenuTogglePlugins: (json['inputMenuTogglePlugins'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      chatInputHooks: (json['chatInputHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      chatViewHooks: (json['chatViewHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      chatMessageHooks: (json['chatMessageHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      hostEventHooks: (json['hostEventHooks'] as List<Object?>).map((item) => ToolPkgHostEventHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      toolLifecycleHooks: (json['toolLifecycleHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      promptInputHooks: (json['promptInputHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      promptHistoryHooks: (json['promptHistoryHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      promptEstimateHistoryHooks: (json['promptEstimateHistoryHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      systemPromptComposeHooks: (json['systemPromptComposeHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      toolPromptComposeHooks: (json['toolPromptComposeHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      promptFinalizeHooks: (json['promptFinalizeHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      promptEstimateFinalizeHooks: (json['promptEstimateFinalizeHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      summaryGenerateHooks: (json['summaryGenerateHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
      aiProviders: (json['aiProviders'] as List<Object?>).map((item) => ToolPkgAiProviderRuntime.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'packageName': packageName,
      'displayName': displayName.toJson(),
      'description': description.toJson(),
      'version': version,
      'author': author.map((item) => item).toList(growable: false),
      'mainEntry': mainEntry,
      'sourceType': sourceType.toJson(),
      'sourcePath': sourcePath,
      'subpackages': subpackages.map((item) => item.toJson()).toList(growable: false),
      'resources': resources.map((item) => item.toJson()).toList(growable: false),
      'wasmModules': wasmModules.map((item) => item.toJson()).toList(growable: false),
      'workflowTemplates': workflowTemplates.map((item) => item.toJson()).toList(growable: false),
      'workspaceTemplates': workspaceTemplates.map((item) => item.toJson()).toList(growable: false),
      'uiModules': uiModules.map((item) => item.toJson()).toList(growable: false),
      'uiRoutes': uiRoutes.map((item) => item.toJson()).toList(growable: false),
      'navigationEntries': navigationEntries.map((item) => item.toJson()).toList(growable: false),
      'desktopWidgets': desktopWidgets.map((item) => item.toJson()).toList(growable: false),
      'appLifecycleHooks': appLifecycleHooks.map((item) => item.toJson()).toList(growable: false),
      'messageProcessingPlugins': messageProcessingPlugins.map((item) => item.toJson()).toList(growable: false),
      'xmlRenderPlugins': xmlRenderPlugins.map((item) => item.toJson()).toList(growable: false),
      'inputMenuTogglePlugins': inputMenuTogglePlugins.map((item) => item.toJson()).toList(growable: false),
      'chatInputHooks': chatInputHooks.map((item) => item.toJson()).toList(growable: false),
      'chatViewHooks': chatViewHooks.map((item) => item.toJson()).toList(growable: false),
      'chatMessageHooks': chatMessageHooks.map((item) => item.toJson()).toList(growable: false),
      'hostEventHooks': hostEventHooks.map((item) => item.toJson()).toList(growable: false),
      'toolLifecycleHooks': toolLifecycleHooks.map((item) => item.toJson()).toList(growable: false),
      'promptInputHooks': promptInputHooks.map((item) => item.toJson()).toList(growable: false),
      'promptHistoryHooks': promptHistoryHooks.map((item) => item.toJson()).toList(growable: false),
      'promptEstimateHistoryHooks': promptEstimateHistoryHooks.map((item) => item.toJson()).toList(growable: false),
      'systemPromptComposeHooks': systemPromptComposeHooks.map((item) => item.toJson()).toList(growable: false),
      'toolPromptComposeHooks': toolPromptComposeHooks.map((item) => item.toJson()).toList(growable: false),
      'promptFinalizeHooks': promptFinalizeHooks.map((item) => item.toJson()).toList(growable: false),
      'promptEstimateFinalizeHooks': promptEstimateFinalizeHooks.map((item) => item.toJson()).toList(growable: false),
      'summaryGenerateHooks': summaryGenerateHooks.map((item) => item.toJson()).toList(growable: false),
      'aiProviders': aiProviders.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `packageName` serialized as `packageName`.
  final String packageName;
  /// Rust field `displayName` serialized as `displayName`.
  final LocalizedText displayName;
  /// Rust field `description` serialized as `description`.
  final LocalizedText description;
  /// Rust field `version` serialized as `version`.
  final String version;
  /// Rust field `author` serialized as `author`.
  final List<String> author;
  /// Rust field `mainEntry` serialized as `mainEntry`.
  final String mainEntry;
  /// Rust field `sourceType` serialized as `sourceType`.
  final ToolPkgSourceType sourceType;
  /// Rust field `sourcePath` serialized as `sourcePath`.
  final String sourcePath;
  /// Rust field `subpackages` serialized as `subpackages`.
  final List<ToolPkgSubpackageRuntime> subpackages;
  /// Rust field `resources` serialized as `resources`.
  final List<ToolPkgResourceRuntime> resources;
  /// Rust field `wasmModules` serialized as `wasmModules`.
  final List<ToolPkgWasmModuleRuntime> wasmModules;
  /// Rust field `workflowTemplates` serialized as `workflowTemplates`.
  final List<ToolPkgWorkflowTemplateRuntime> workflowTemplates;
  /// Rust field `workspaceTemplates` serialized as `workspaceTemplates`.
  final List<ToolPkgWorkspaceTemplateRuntime> workspaceTemplates;
  /// Rust field `uiModules` serialized as `uiModules`.
  final List<ToolPkgUiModuleRuntime> uiModules;
  /// Rust field `uiRoutes` serialized as `uiRoutes`.
  final List<ToolPkgUiRouteRuntime> uiRoutes;
  /// Rust field `navigationEntries` serialized as `navigationEntries`.
  final List<ToolPkgNavigationEntryRuntime> navigationEntries;
  /// Rust field `desktopWidgets` serialized as `desktopWidgets`.
  final List<ToolPkgDesktopWidgetRuntime> desktopWidgets;
  /// Rust field `appLifecycleHooks` serialized as `appLifecycleHooks`.
  final List<ToolPkgAppLifecycleHookRuntime> appLifecycleHooks;
  /// Rust field `messageProcessingPlugins` serialized as `messageProcessingPlugins`.
  final List<ToolPkgFunctionHookRuntime> messageProcessingPlugins;
  /// Rust field `xmlRenderPlugins` serialized as `xmlRenderPlugins`.
  final List<ToolPkgTagFunctionHookRuntime> xmlRenderPlugins;
  /// Rust field `inputMenuTogglePlugins` serialized as `inputMenuTogglePlugins`.
  final List<ToolPkgFunctionHookRuntime> inputMenuTogglePlugins;
  /// Rust field `chatInputHooks` serialized as `chatInputHooks`.
  final List<ToolPkgFunctionHookRuntime> chatInputHooks;
  /// Rust field `chatViewHooks` serialized as `chatViewHooks`.
  final List<ToolPkgFunctionHookRuntime> chatViewHooks;
  /// Rust field `chatMessageHooks` serialized as `chatMessageHooks`.
  final List<ToolPkgFunctionHookRuntime> chatMessageHooks;
  /// Rust field `hostEventHooks` serialized as `hostEventHooks`.
  final List<ToolPkgHostEventHookRuntime> hostEventHooks;
  /// Rust field `toolLifecycleHooks` serialized as `toolLifecycleHooks`.
  final List<ToolPkgFunctionHookRuntime> toolLifecycleHooks;
  /// Rust field `promptInputHooks` serialized as `promptInputHooks`.
  final List<ToolPkgFunctionHookRuntime> promptInputHooks;
  /// Rust field `promptHistoryHooks` serialized as `promptHistoryHooks`.
  final List<ToolPkgFunctionHookRuntime> promptHistoryHooks;
  /// Rust field `promptEstimateHistoryHooks` serialized as `promptEstimateHistoryHooks`.
  final List<ToolPkgFunctionHookRuntime> promptEstimateHistoryHooks;
  /// Rust field `systemPromptComposeHooks` serialized as `systemPromptComposeHooks`.
  final List<ToolPkgFunctionHookRuntime> systemPromptComposeHooks;
  /// Rust field `toolPromptComposeHooks` serialized as `toolPromptComposeHooks`.
  final List<ToolPkgFunctionHookRuntime> toolPromptComposeHooks;
  /// Rust field `promptFinalizeHooks` serialized as `promptFinalizeHooks`.
  final List<ToolPkgFunctionHookRuntime> promptFinalizeHooks;
  /// Rust field `promptEstimateFinalizeHooks` serialized as `promptEstimateFinalizeHooks`.
  final List<ToolPkgFunctionHookRuntime> promptEstimateFinalizeHooks;
  /// Rust field `summaryGenerateHooks` serialized as `summaryGenerateHooks`.
  final List<ToolPkgFunctionHookRuntime> summaryGenerateHooks;
  /// Rust field `aiProviders` serialized as `aiProviders`.
  final List<ToolPkgAiProviderRuntime> aiProviders;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgDesktopWidgetRuntime`.
class ToolPkgDesktopWidgetRuntime {
  const ToolPkgDesktopWidgetRuntime({
    required this.id,
    required this.routeId,
    required this.renderRouteId,
    required this.title,
    required this.subtitle,
    required this.description,
    required this.icon,
    required this.order,
  });

  factory ToolPkgDesktopWidgetRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgDesktopWidgetRuntime(
      id: json['id'] as String,
      routeId: json['routeId'] as String,
      renderRouteId: json['renderRouteId'] as String,
      title: LocalizedText.fromJson(json['title'] as Map<String, Object?>),
      subtitle: LocalizedText.fromJson(json['subtitle'] as Map<String, Object?>),
      description: LocalizedText.fromJson(json['description'] as Map<String, Object?>),
      icon: json['icon'] == null ? null : json['icon'] as String,
      order: json['order'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'routeId': routeId,
      'renderRouteId': renderRouteId,
      'title': title.toJson(),
      'subtitle': subtitle.toJson(),
      'description': description.toJson(),
      'icon': icon,
      'order': order,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `routeId` serialized as `routeId`.
  final String routeId;
  /// Rust field `renderRouteId` serialized as `renderRouteId`.
  final String renderRouteId;
  /// Rust field `title` serialized as `title`.
  final LocalizedText title;
  /// Rust field `subtitle` serialized as `subtitle`.
  final LocalizedText subtitle;
  /// Rust field `description` serialized as `description`.
  final LocalizedText description;
  /// Rust field `icon` serialized as `icon`.
  final String? icon;
  /// Rust field `order` serialized as `order`.
  final int order;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgFunctionHookRuntime`.
class ToolPkgFunctionHookRuntime {
  const ToolPkgFunctionHookRuntime({
    required this.id,
    required this.function,
    required this.functionSource,
  });

  factory ToolPkgFunctionHookRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgFunctionHookRuntime(
      id: json['id'] as String,
      function: json['function'] as String,
      functionSource: json['functionSource'] == null ? null : json['functionSource'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'function': function,
      'functionSource': functionSource,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `function` serialized as `function`.
  final String function;
  /// Rust field `functionSource` serialized as `functionSource`.
  final String? functionSource;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgHostEventHookRuntime`.
class ToolPkgHostEventHookRuntime {
  const ToolPkgHostEventHookRuntime({
    required this.id,
    required this.source,
    required this.trigger,
    required this.function,
    required this.functionSource,
    required this.enabled,
  });

  factory ToolPkgHostEventHookRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgHostEventHookRuntime(
      id: json['id'] as String,
      source: json['source'] as String,
      trigger: json['trigger'],
      function: json['function'] as String,
      functionSource: json['functionSource'] == null ? null : json['functionSource'] as String,
      enabled: json['enabled'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'source': source,
      'trigger': trigger,
      'function': function,
      'functionSource': functionSource,
      'enabled': enabled,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `source` serialized as `source`.
  final String source;
  /// Rust field `trigger` serialized as `trigger`.
  final Object? trigger;
  /// Rust field `function` serialized as `function`.
  final String function;
  /// Rust field `functionSource` serialized as `functionSource`.
  final String? functionSource;
  /// Rust field `enabled` serialized as `enabled`.
  final bool enabled;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgLoadResult`.
class ToolPkgLoadResult {
  const ToolPkgLoadResult({
    required this.containerPackage,
    required this.subpackagePackages,
    required this.containerRuntime,
    required this.marketOrigin,
  });

  factory ToolPkgLoadResult.fromJson(Map<String, Object?> json) {
    return ToolPkgLoadResult(
      containerPackage: ToolPackage.fromJson(json['containerPackage'] as Map<String, Object?>),
      subpackagePackages: (json['subpackagePackages'] as List<Object?>).map((item) => ToolPackage.fromJson(item as Map<String, Object?>)).toList(growable: false),
      containerRuntime: ToolPkgContainerRuntime.fromJson(json['containerRuntime'] as Map<String, Object?>),
      marketOrigin: json['marketOrigin'] == null ? null : ToolPkgMarketOrigin.fromJson(json['marketOrigin'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'containerPackage': containerPackage.toJson(),
      'subpackagePackages': subpackagePackages.map((item) => item.toJson()).toList(growable: false),
      'containerRuntime': containerRuntime.toJson(),
      'marketOrigin': marketOrigin?.toJson(),
    };
  }

  /// Rust field `containerPackage` serialized as `containerPackage`.
  final ToolPackage containerPackage;
  /// Rust field `subpackagePackages` serialized as `subpackagePackages`.
  final List<ToolPackage> subpackagePackages;
  /// Rust field `containerRuntime` serialized as `containerRuntime`.
  final ToolPkgContainerRuntime containerRuntime;
  /// Rust field `marketOrigin` serialized as `marketOrigin`.
  final ToolPkgMarketOrigin? marketOrigin;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgMarketOrigin`.
class ToolPkgMarketOrigin {
  const ToolPkgMarketOrigin({
    required this.market,
    required this.toolpkgId,
    required this.version,
    required this.author,
  });

  factory ToolPkgMarketOrigin.fromJson(Map<String, Object?> json) {
    return ToolPkgMarketOrigin(
      market: json['market'] as String,
      toolpkgId: json['toolpkgId'] as String,
      version: json['version'] as String,
      author: (json['author'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'market': market,
      'toolpkgId': toolpkgId,
      'version': version,
      'author': author.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `market` serialized as `market`.
  final String market;
  /// Rust field `toolpkgId` serialized as `toolpkgId`.
  final String toolpkgId;
  /// Rust field `version` serialized as `version`.
  final String version;
  /// Rust field `author` serialized as `author`.
  final List<String> author;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgNavigationActionHookRuntime`.
class ToolPkgNavigationActionHookRuntime {
  const ToolPkgNavigationActionHookRuntime({
    required this.function,
    required this.functionSource,
  });

  factory ToolPkgNavigationActionHookRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgNavigationActionHookRuntime(
      function: json['function'] as String,
      functionSource: json['functionSource'] == null ? null : json['functionSource'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'function': function,
      'functionSource': functionSource,
    };
  }

  /// Rust field `function` serialized as `function`.
  final String function;
  /// Rust field `functionSource` serialized as `functionSource`.
  final String? functionSource;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgNavigationEntryRuntime`.
class ToolPkgNavigationEntryRuntime {
  const ToolPkgNavigationEntryRuntime({
    required this.id,
    required this.routeId,
    required this.surface,
    required this.title,
    required this.action,
    required this.icon,
    required this.order,
  });

  factory ToolPkgNavigationEntryRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgNavigationEntryRuntime(
      id: json['id'] as String,
      routeId: json['routeId'] as String,
      surface: json['surface'] as String,
      title: LocalizedText.fromJson(json['title'] as Map<String, Object?>),
      action: json['action'] == null ? null : ToolPkgNavigationActionHookRuntime.fromJson(json['action'] as Map<String, Object?>),
      icon: json['icon'] == null ? null : json['icon'] as String,
      order: json['order'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'routeId': routeId,
      'surface': surface,
      'title': title.toJson(),
      'action': action?.toJson(),
      'icon': icon,
      'order': order,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `routeId` serialized as `routeId`.
  final String routeId;
  /// Rust field `surface` serialized as `surface`.
  final String surface;
  /// Rust field `title` serialized as `title`.
  final LocalizedText title;
  /// Rust field `action` serialized as `action`.
  final ToolPkgNavigationActionHookRuntime? action;
  /// Rust field `icon` serialized as `icon`.
  final String? icon;
  /// Rust field `order` serialized as `order`.
  final int order;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgResourceRuntime`.
class ToolPkgResourceRuntime {
  const ToolPkgResourceRuntime({
    required this.key,
    required this.path,
    required this.mime,
  });

  factory ToolPkgResourceRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgResourceRuntime(
      key: json['key'] as String,
      path: json['path'] as String,
      mime: json['mime'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'key': key,
      'path': path,
      'mime': mime,
    };
  }

  /// Rust field `key` serialized as `key`.
  final String key;
  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `mime` serialized as `mime`.
  final String mime;
}

/// Generated Dart enum for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgSourceType`.
enum ToolPkgSourceType {
  asset('ASSET'),
  market('MARKET'),
  externalValue('EXTERNAL'),
  ;

  const ToolPkgSourceType(this.value);

  final String value;

  factory ToolPkgSourceType.fromJson(Object? value) {
    return switch (value) {
      'ASSET' => ToolPkgSourceType.asset,
      'MARKET' => ToolPkgSourceType.market,
      'EXTERNAL' => ToolPkgSourceType.externalValue,
      _ => throw ArgumentError('Unknown ToolPkgSourceType: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgSubpackageRuntime`.
class ToolPkgSubpackageRuntime {
  const ToolPkgSubpackageRuntime({
    required this.packageName,
    required this.containerPackageName,
    required this.subpackageId,
    required this.entryPath,
    required this.displayName,
    required this.description,
    required this.enabledByDefault,
    required this.toolCount,
  });

  factory ToolPkgSubpackageRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgSubpackageRuntime(
      packageName: json['packageName'] as String,
      containerPackageName: json['containerPackageName'] as String,
      subpackageId: json['subpackageId'] as String,
      entryPath: json['entryPath'] as String,
      displayName: LocalizedText.fromJson(json['displayName'] as Map<String, Object?>),
      description: LocalizedText.fromJson(json['description'] as Map<String, Object?>),
      enabledByDefault: json['enabledByDefault'] as bool,
      toolCount: json['toolCount'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'packageName': packageName,
      'containerPackageName': containerPackageName,
      'subpackageId': subpackageId,
      'entryPath': entryPath,
      'displayName': displayName.toJson(),
      'description': description.toJson(),
      'enabledByDefault': enabledByDefault,
      'toolCount': toolCount,
    };
  }

  /// Rust field `packageName` serialized as `packageName`.
  final String packageName;
  /// Rust field `containerPackageName` serialized as `containerPackageName`.
  final String containerPackageName;
  /// Rust field `subpackageId` serialized as `subpackageId`.
  final String subpackageId;
  /// Rust field `entryPath` serialized as `entryPath`.
  final String entryPath;
  /// Rust field `displayName` serialized as `displayName`.
  final LocalizedText displayName;
  /// Rust field `description` serialized as `description`.
  final LocalizedText description;
  /// Rust field `enabledByDefault` serialized as `enabledByDefault`.
  final bool enabledByDefault;
  /// Rust field `toolCount` serialized as `toolCount`.
  final int toolCount;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgTagFunctionHookRuntime`.
class ToolPkgTagFunctionHookRuntime {
  const ToolPkgTagFunctionHookRuntime({
    required this.id,
    required this.tag,
    required this.function,
    required this.functionSource,
  });

  factory ToolPkgTagFunctionHookRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgTagFunctionHookRuntime(
      id: json['id'] as String,
      tag: json['tag'] as String,
      function: json['function'] as String,
      functionSource: json['functionSource'] == null ? null : json['functionSource'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'tag': tag,
      'function': function,
      'functionSource': functionSource,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `tag` serialized as `tag`.
  final String tag;
  /// Rust field `function` serialized as `function`.
  final String function;
  /// Rust field `functionSource` serialized as `functionSource`.
  final String? functionSource;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgUiModuleRuntime`.
class ToolPkgUiModuleRuntime {
  const ToolPkgUiModuleRuntime({
    required this.id,
    required this.runtime,
    required this.screen,
    required this.title,
    required this.keepAlive,
  });

  factory ToolPkgUiModuleRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgUiModuleRuntime(
      id: json['id'] as String,
      runtime: json['runtime'] as String,
      screen: json['screen'] as String,
      title: LocalizedText.fromJson(json['title'] as Map<String, Object?>),
      keepAlive: json['keepAlive'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'runtime': runtime,
      'screen': screen,
      'title': title.toJson(),
      'keepAlive': keepAlive,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `runtime` serialized as `runtime`.
  final String runtime;
  /// Rust field `screen` serialized as `screen`.
  final String screen;
  /// Rust field `title` serialized as `title`.
  final LocalizedText title;
  /// Rust field `keepAlive` serialized as `keepAlive`.
  final bool keepAlive;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgUiRouteRuntime`.
class ToolPkgUiRouteRuntime {
  const ToolPkgUiRouteRuntime({
    required this.id,
    required this.routeId,
    required this.runtime,
    required this.screen,
    required this.title,
    required this.keepAlive,
  });

  factory ToolPkgUiRouteRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgUiRouteRuntime(
      id: json['id'] as String,
      routeId: json['routeId'] as String,
      runtime: json['runtime'] as String,
      screen: json['screen'] as String,
      title: LocalizedText.fromJson(json['title'] as Map<String, Object?>),
      keepAlive: json['keepAlive'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'routeId': routeId,
      'runtime': runtime,
      'screen': screen,
      'title': title.toJson(),
      'keepAlive': keepAlive,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `routeId` serialized as `routeId`.
  final String routeId;
  /// Rust field `runtime` serialized as `runtime`.
  final String runtime;
  /// Rust field `screen` serialized as `screen`.
  final String screen;
  /// Rust field `title` serialized as `title`.
  final LocalizedText title;
  /// Rust field `keepAlive` serialized as `keepAlive`.
  final bool keepAlive;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgWasmModuleRuntime`.
class ToolPkgWasmModuleRuntime {
  const ToolPkgWasmModuleRuntime({
    required this.id,
    required this.path,
    required this.exports,
    required this.sourceLanguage,
    required this.abi,
  });

  factory ToolPkgWasmModuleRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgWasmModuleRuntime(
      id: json['id'] as String,
      path: json['path'] as String,
      exports: (json['exports'] as List<Object?>).map((item) => item as String).toList(growable: false),
      sourceLanguage: json['sourceLanguage'] as String,
      abi: json['abi'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'path': path,
      'exports': exports.map((item) => item).toList(growable: false),
      'sourceLanguage': sourceLanguage,
      'abi': abi,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `exports` serialized as `exports`.
  final List<String> exports;
  /// Rust field `sourceLanguage` serialized as `sourceLanguage`.
  final String sourceLanguage;
  /// Rust field `abi` serialized as `abi`.
  final String abi;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgTemplateModels::ToolPkgWorkflowTemplateRuntime`.
class ToolPkgWorkflowTemplateRuntime {
  const ToolPkgWorkflowTemplateRuntime({
    required this.id,
    required this.displayName,
    required this.description,
    required this.resourceKey,
  });

  factory ToolPkgWorkflowTemplateRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgWorkflowTemplateRuntime(
      id: json['id'] as String,
      displayName: LocalizedText.fromJson(json['display_name'] as Map<String, Object?>),
      description: LocalizedText.fromJson(json['description'] as Map<String, Object?>),
      resourceKey: json['resource_key'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'display_name': displayName.toJson(),
      'description': description.toJson(),
      'resource_key': resourceKey,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `display_name` serialized as `display_name`.
  final LocalizedText displayName;
  /// Rust field `description` serialized as `description`.
  final LocalizedText description;
  /// Rust field `resource_key` serialized as `resource_key`.
  final String resourceKey;
}

/// Generated Dart model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgTemplateModels::ToolPkgWorkspaceTemplateRuntime`.
class ToolPkgWorkspaceTemplateRuntime {
  const ToolPkgWorkspaceTemplateRuntime({
    required this.id,
    required this.displayName,
    required this.description,
    required this.resourceKey,
    required this.projectType,
  });

  factory ToolPkgWorkspaceTemplateRuntime.fromJson(Map<String, Object?> json) {
    return ToolPkgWorkspaceTemplateRuntime(
      id: json['id'] as String,
      displayName: LocalizedText.fromJson(json['display_name'] as Map<String, Object?>),
      description: LocalizedText.fromJson(json['description'] as Map<String, Object?>),
      resourceKey: json['resource_key'] as String,
      projectType: json['project_type'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'display_name': displayName.toJson(),
      'description': description.toJson(),
      'resource_key': resourceKey,
      'project_type': projectType,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `display_name` serialized as `display_name`.
  final LocalizedText displayName;
  /// Rust field `description` serialized as `description`.
  final LocalizedText description;
  /// Rust field `resource_key` serialized as `resource_key`.
  final String resourceKey;
  /// Rust field `project_type` serialized as `project_type`.
  final String projectType;
}

/// Generated Dart model for Rust type `operit_providers::chat::llmprovider::ModelConfigConnectionTester::ModelConnectionTestItem`.
class CoreOperitProvidersChatLlmproviderModelConfigConnectionTesterModelConnectionTestItem {
  const CoreOperitProvidersChatLlmproviderModelConfigConnectionTesterModelConnectionTestItem({
    required this.type,
    required this.success,
    required this.error,
  });

  factory CoreOperitProvidersChatLlmproviderModelConfigConnectionTesterModelConnectionTestItem.fromJson(Map<String, Object?> json) {
    return CoreOperitProvidersChatLlmproviderModelConfigConnectionTesterModelConnectionTestItem(
      type: ModelConnectionTestType.fromJson(json['type']),
      success: json['success'] as bool,
      error: json['error'] == null ? null : json['error'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'type': type.toJson(),
      'success': success,
      'error': error,
    };
  }

  /// Rust field `r#type` serialized as `type`.
  final ModelConnectionTestType type;
  /// Rust field `success` serialized as `success`.
  final bool success;
  /// Rust field `error` serialized as `error`.
  final String? error;
}

/// Generated Dart model for Rust type `operit_providers::chat::llmprovider::ModelConfigConnectionTester::ModelConnectionTestReport`.
class ModelConnectionTestReport {
  const ModelConnectionTestReport({
    required this.providerId,
    required this.modelId,
    required this.providerName,
    required this.providerType,
    required this.success,
    required this.items,
  });

  factory ModelConnectionTestReport.fromJson(Map<String, Object?> json) {
    return ModelConnectionTestReport(
      providerId: json['providerId'] as String,
      modelId: json['modelId'] as String,
      providerName: json['providerName'] as String,
      providerType: json['providerType'] as String,
      success: json['success'] as bool,
      items: (json['items'] as List<Object?>).map((item) => CoreOperitProvidersChatLlmproviderModelConfigConnectionTesterModelConnectionTestItem.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'providerId': providerId,
      'modelId': modelId,
      'providerName': providerName,
      'providerType': providerType,
      'success': success,
      'items': items.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `providerId` serialized as `providerId`.
  final String providerId;
  /// Rust field `modelId` serialized as `modelId`.
  final String modelId;
  /// Rust field `providerName` serialized as `providerName`.
  final String providerName;
  /// Rust field `providerType` serialized as `providerType`.
  final String providerType;
  /// Rust field `success` serialized as `success`.
  final bool success;
  /// Rust field `items` serialized as `items`.
  final List<CoreOperitProvidersChatLlmproviderModelConfigConnectionTesterModelConnectionTestItem> items;
}

/// Generated Dart enum for Rust type `operit_providers::chat::llmprovider::ModelConfigConnectionTester::ModelConnectionTestType`.
enum ModelConnectionTestType {
  chat('CHAT'),
  toolCall('TOOL_CALL'),
  image('IMAGE'),
  audio('AUDIO'),
  video('VIDEO'),
  ;

  const ModelConnectionTestType(this.value);

  final String value;

  factory ModelConnectionTestType.fromJson(Object? value) {
    return switch (value) {
      'CHAT' => ModelConnectionTestType.chat,
      'TOOL_CALL' => ModelConnectionTestType.toolCall,
      'IMAGE' => ModelConnectionTestType.image,
      'AUDIO' => ModelConnectionTestType.audio,
      'VIDEO' => ModelConnectionTestType.video,
      _ => throw ArgumentError('Unknown ModelConnectionTestType: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::GitHubUser`.
class CoreOperitProvidersMarketMarketStatsApiServiceGitHubUser {
  const CoreOperitProvidersMarketMarketStatsApiServiceGitHubUser({
    required this.id,
    required this.login,
    required this.name,
    required this.email,
    required this.avatarUrl,
    required this.bio,
    required this.publicRepos,
    required this.followers,
    required this.following,
  });

  factory CoreOperitProvidersMarketMarketStatsApiServiceGitHubUser.fromJson(Map<String, Object?> json) {
    return CoreOperitProvidersMarketMarketStatsApiServiceGitHubUser(
      id: json['id'] as int,
      login: json['login'] as String,
      name: json['name'] == null ? null : json['name'] as String,
      email: json['email'] == null ? null : json['email'] as String,
      avatarUrl: json['avatar_url'] as String,
      bio: json['bio'] == null ? null : json['bio'] as String,
      publicRepos: json['public_repos'] == null ? null : json['public_repos'] as int,
      followers: json['followers'] == null ? null : json['followers'] as int,
      following: json['following'] == null ? null : json['following'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'login': login,
      'name': name,
      'email': email,
      'avatar_url': avatarUrl,
      'bio': bio,
      'public_repos': publicRepos,
      'followers': followers,
      'following': following,
    };
  }

  /// Rust field `id` serialized as `id`.
  final int id;
  /// Rust field `login` serialized as `login`.
  final String login;
  /// Rust field `name` serialized as `name`.
  final String? name;
  /// Rust field `email` serialized as `email`.
  final String? email;
  /// Rust field `avatar_url` serialized as `avatar_url`.
  final String avatarUrl;
  /// Rust field `bio` serialized as `bio`.
  final String? bio;
  /// Rust field `public_repos` serialized as `public_repos`.
  final int? publicRepos;
  /// Rust field `followers` serialized as `followers`.
  final int? followers;
  /// Rust field `following` serialized as `following`.
  final int? following;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketAuthInfo`.
class MarketAuthInfo {
  const MarketAuthInfo({
    required this.ok,
    required this.session,
    required this.githubId,
    required this.login,
    required this.avatarUrl,
  });

  factory MarketAuthInfo.fromJson(Map<String, Object?> json) {
    return MarketAuthInfo(
      ok: json['ok'] as bool,
      session: json['session'] as String,
      githubId: json['githubId'] as int,
      login: json['login'] as String,
      avatarUrl: json['avatarUrl'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'ok': ok,
      'session': session,
      'githubId': githubId,
      'login': login,
      'avatarUrl': avatarUrl,
    };
  }

  /// Rust field `ok` serialized as `ok`.
  final bool ok;
  /// Rust field `session` serialized as `session`.
  final String session;
  /// Rust field `github_id` serialized as `githubId`.
  final int githubId;
  /// Rust field `login` serialized as `login`.
  final String login;
  /// Rust field `avatar_url` serialized as `avatarUrl`.
  final String avatarUrl;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketAuthor`.
class MarketAuthor {
  const MarketAuthor({
    required this.id,
    required this.githubId,
    required this.login,
    required this.avatar,
    required this.status,
  });

  factory MarketAuthor.fromJson(Map<String, Object?> json) {
    return MarketAuthor(
      id: json['id'] as String,
      githubId: json['githubId'] as int,
      login: json['login'] as String,
      avatar: json['avatar'] as String,
      status: json['status'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'githubId': githubId,
      'login': login,
      'avatar': avatar,
      'status': status,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `github_id` serialized as `githubId`.
  final int githubId;
  /// Rust field `login` serialized as `login`.
  final String login;
  /// Rust field `avatar` serialized as `avatar`.
  final String avatar;
  /// Rust field `status` serialized as `status`.
  final String status;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketCategoryInfo`.
class MarketCategoryInfo {
  const MarketCategoryInfo({
    required this.id,
    required this.name,
    required this.description,
  });

  factory MarketCategoryInfo.fromJson(Map<String, Object?> json) {
    return MarketCategoryInfo(
      id: json['id'] as String,
      name: json['name'] as String,
      description: json['description'] == null ? null : json['description'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
      'description': description,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final String? description;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketComment`.
class MarketComment {
  const MarketComment({
    required this.id,
    required this.entryId,
    required this.parentId,
    required this.author,
    required this.body,
    required this.createdAt,
    required this.updatedAt,
  });

  factory MarketComment.fromJson(Map<String, Object?> json) {
    return MarketComment(
      id: json['id'] as String,
      entryId: json['entryId'] as String,
      parentId: json['parentId'] == null ? null : json['parentId'] as String,
      author: MarketAuthor.fromJson(json['author'] as Map<String, Object?>),
      body: json['body'] as String,
      createdAt: json['createdAt'] as String,
      updatedAt: json['updatedAt'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'entryId': entryId,
      'parentId': parentId,
      'author': author.toJson(),
      'body': body,
      'createdAt': createdAt,
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `entry_id` serialized as `entryId`.
  final String entryId;
  /// Rust field `parent_id` serialized as `parentId`.
  final String? parentId;
  /// Rust field `author` serialized as `author`.
  final MarketAuthor author;
  /// Rust field `body` serialized as `body`.
  final String body;
  /// Rust field `created_at` serialized as `createdAt`.
  final String createdAt;
  /// Rust field `updated_at` serialized as `updatedAt`.
  final String updatedAt;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketCommentPage`.
class MarketCommentPage {
  const MarketCommentPage({
    required this.ok,
    required this.entryId,
    required this.page,
    required this.pageSize,
    required this.total,
    required this.items,
    required this.generatedAt,
  });

  factory MarketCommentPage.fromJson(Map<String, Object?> json) {
    return MarketCommentPage(
      ok: json['ok'] as bool,
      entryId: json['entryId'] as String,
      page: json['page'] as int,
      pageSize: json['pageSize'] as int,
      total: json['total'] as int,
      items: (json['items'] as List<Object?>).map((item) => MarketComment.fromJson(item as Map<String, Object?>)).toList(growable: false),
      generatedAt: json['generatedAt'] == null ? null : json['generatedAt'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'ok': ok,
      'entryId': entryId,
      'page': page,
      'pageSize': pageSize,
      'total': total,
      'items': items.map((item) => item.toJson()).toList(growable: false),
      'generatedAt': generatedAt,
    };
  }

  /// Rust field `ok` serialized as `ok`.
  final bool ok;
  /// Rust field `entry_id` serialized as `entryId`.
  final String entryId;
  /// Rust field `page` serialized as `page`.
  final int page;
  /// Rust field `page_size` serialized as `pageSize`.
  final int pageSize;
  /// Rust field `total` serialized as `total`.
  final int total;
  /// Rust field `items` serialized as `items`.
  final List<MarketComment> items;
  /// Rust field `generated_at` serialized as `generatedAt`.
  final String? generatedAt;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketEntriesShard`.
class MarketEntriesShard {
  const MarketEntriesShard({
    required this.generatedAt,
    required this.entriesById,
  });

  factory MarketEntriesShard.fromJson(Map<String, Object?> json) {
    return MarketEntriesShard(
      generatedAt: json['generatedAt'] == null ? null : json['generatedAt'] as String,
      entriesById: (json['entriesById'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, MarketEntrySummary.fromJson(value as Map<String, Object?>))),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'generatedAt': generatedAt,
      'entriesById': entriesById.map((key, value) => MapEntry(key, value.toJson())),
    };
  }

  /// Rust field `generated_at` serialized as `generatedAt`.
  final String? generatedAt;
  /// Rust field `entries_by_id` serialized as `entriesById`.
  final Map<String, MarketEntrySummary> entriesById;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketEntryArtifact`.
class MarketEntryArtifact {
  const MarketEntryArtifact({
    required this.projectId,
    required this.runtimePackageId,
  });

  factory MarketEntryArtifact.fromJson(Map<String, Object?> json) {
    return MarketEntryArtifact(
      projectId: json['projectId'] as String,
      runtimePackageId: json['runtimePackageId'] == null ? null : json['runtimePackageId'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'projectId': projectId,
      'runtimePackageId': runtimePackageId,
    };
  }

  /// Rust field `project_id` serialized as `projectId`.
  final String projectId;
  /// Rust field `runtime_package_id` serialized as `runtimePackageId`.
  final String? runtimePackageId;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketEntryAsset`.
class MarketEntryAsset {
  const MarketEntryAsset({
    required this.id,
    required this.versionId,
    required this.kind,
    required this.url,
    required this.sha256,
    required this.assetName,
  });

  factory MarketEntryAsset.fromJson(Map<String, Object?> json) {
    return MarketEntryAsset(
      id: json['id'] as String,
      versionId: json['versionId'] as String,
      kind: json['kind'] as String,
      url: json['url'] as String,
      sha256: json['sha256'] as String,
      assetName: json['assetName'] == null ? null : json['assetName'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'versionId': versionId,
      'kind': kind,
      'url': url,
      'sha256': sha256,
      'assetName': assetName,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `version_id` serialized as `versionId`.
  final String versionId;
  /// Rust field `kind` serialized as `kind`.
  final String kind;
  /// Rust field `url` serialized as `url`.
  final String url;
  /// Rust field `sha256` serialized as `sha256`.
  final String sha256;
  /// Rust field `asset_name` serialized as `assetName`.
  final String? assetName;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketEntrySummary`.
class MarketEntrySummary {
  const MarketEntrySummary({
    required this.type,
    required this.id,
    required this.title,
    required this.description,
    required this.detail,
    required this.author,
    required this.publisher,
    required this.contributors,
    required this.categoryId,
    required this.stateCode,
    required this.allowPublicUpdates,
    required this.featured,
    required this.downloads,
    required this.downloadCount,
    required this.createdAt,
    required this.updatedAt,
    required this.publishedAt,
    required this.source,
    required this.repoVersion,
    required this.artifact,
    required this.assets,
    required this.versions,
    required this.latestVersion,
    required this.reactions,
  });

  factory MarketEntrySummary.fromJson(Map<String, Object?> json) {
    return MarketEntrySummary(
      type: json['type'] as String,
      id: json['id'] as String,
      title: json['title'] as String,
      description: json['description'] as String,
      detail: json['detail'] as String,
      author: json['author'] == null ? null : MarketAuthor.fromJson(json['author'] as Map<String, Object?>),
      publisher: json['publisher'] == null ? null : MarketAuthor.fromJson(json['publisher'] as Map<String, Object?>),
      contributors: (json['contributors'] as List<Object?>).map((item) => MarketAuthor.fromJson(item as Map<String, Object?>)).toList(growable: false),
      categoryId: json['categoryId'] == null ? null : json['categoryId'] as String,
      stateCode: json['stateCode'] as String,
      allowPublicUpdates: json['allowPublicUpdates'] as bool,
      featured: json['featured'] as bool,
      downloads: json['downloads'] as int,
      downloadCount: json['downloadCount'] as int,
      createdAt: json['createdAt'] as String,
      updatedAt: json['updatedAt'] as String,
      publishedAt: json['publishedAt'] == null ? null : json['publishedAt'] as String,
      source: json['source'] == null ? null : MarketSource.fromJson(json['source'] as Map<String, Object?>),
      repoVersion: json['repoVersion'] == null ? null : MarketRepoVersion.fromJson(json['repoVersion'] as Map<String, Object?>),
      artifact: json['artifact'] == null ? null : MarketEntryArtifact.fromJson(json['artifact'] as Map<String, Object?>),
      assets: (json['assets'] as List<Object?>).map((item) => MarketEntryAsset.fromJson(item as Map<String, Object?>)).toList(growable: false),
      versions: (json['versions'] as List<Object?>).map((item) => MarketEntryVersion.fromJson(item as Map<String, Object?>)).toList(growable: false),
      latestVersion: json['latestVersion'] == null ? null : MarketEntryVersion.fromJson(json['latestVersion'] as Map<String, Object?>),
      reactions: (json['reactions'] as List<Object?>).map((item) => MarketReactionCount.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'type': type,
      'id': id,
      'title': title,
      'description': description,
      'detail': detail,
      'author': author?.toJson(),
      'publisher': publisher?.toJson(),
      'contributors': contributors.map((item) => item.toJson()).toList(growable: false),
      'categoryId': categoryId,
      'stateCode': stateCode,
      'allowPublicUpdates': allowPublicUpdates,
      'featured': featured,
      'downloads': downloads,
      'downloadCount': downloadCount,
      'createdAt': createdAt,
      'updatedAt': updatedAt,
      'publishedAt': publishedAt,
      'source': source?.toJson(),
      'repoVersion': repoVersion?.toJson(),
      'artifact': artifact?.toJson(),
      'assets': assets.map((item) => item.toJson()).toList(growable: false),
      'versions': versions.map((item) => item.toJson()).toList(growable: false),
      'latestVersion': latestVersion?.toJson(),
      'reactions': reactions.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `r#type` serialized as `type`.
  final String type;
  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `detail` serialized as `detail`.
  final String detail;
  /// Rust field `author` serialized as `author`.
  final MarketAuthor? author;
  /// Rust field `publisher` serialized as `publisher`.
  final MarketAuthor? publisher;
  /// Rust field `contributors` serialized as `contributors`.
  final List<MarketAuthor> contributors;
  /// Rust field `category_id` serialized as `categoryId`.
  final String? categoryId;
  /// Rust field `state_code` serialized as `stateCode`.
  final String stateCode;
  /// Rust field `allow_public_updates` serialized as `allowPublicUpdates`.
  final bool allowPublicUpdates;
  /// Rust field `featured` serialized as `featured`.
  final bool featured;
  /// Rust field `downloads` serialized as `downloads`.
  final int downloads;
  /// Rust field `download_count` serialized as `downloadCount`.
  final int downloadCount;
  /// Rust field `created_at` serialized as `createdAt`.
  final String createdAt;
  /// Rust field `updated_at` serialized as `updatedAt`.
  final String updatedAt;
  /// Rust field `published_at` serialized as `publishedAt`.
  final String? publishedAt;
  /// Rust field `source` serialized as `source`.
  final MarketSource? source;
  /// Rust field `repo_version` serialized as `repoVersion`.
  final MarketRepoVersion? repoVersion;
  /// Rust field `artifact` serialized as `artifact`.
  final MarketEntryArtifact? artifact;
  /// Rust field `assets` serialized as `assets`.
  final List<MarketEntryAsset> assets;
  /// Rust field `versions` serialized as `versions`.
  final List<MarketEntryVersion> versions;
  /// Rust field `latest_version` serialized as `latestVersion`.
  final MarketEntryVersion? latestVersion;
  /// Rust field `reactions` serialized as `reactions`.
  final List<MarketReactionCount> reactions;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketEntryUpdateItem`.
class MarketEntryUpdateItem {
  const MarketEntryUpdateItem({
    required this.id,
    required this.stateCode,
  });

  factory MarketEntryUpdateItem.fromJson(Map<String, Object?> json) {
    return MarketEntryUpdateItem(
      id: json['id'] as String,
      stateCode: json['stateCode'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'stateCode': stateCode,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `state_code` serialized as `stateCode`.
  final String stateCode;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketEntryUpdateResponse`.
class MarketEntryUpdateResponse {
  const MarketEntryUpdateResponse({
    required this.ok,
    required this.item,
    required this.stats,
  });

  factory MarketEntryUpdateResponse.fromJson(Map<String, Object?> json) {
    return MarketEntryUpdateResponse(
      ok: json['ok'] as bool,
      item: MarketEntryUpdateItem.fromJson(json['item'] as Map<String, Object?>),
      stats: (json['stats'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value)),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'ok': ok,
      'item': item.toJson(),
      'stats': stats.map((key, value) => MapEntry(key, value)),
    };
  }

  /// Rust field `ok` serialized as `ok`.
  final bool ok;
  /// Rust field `item` serialized as `item`.
  final MarketEntryUpdateItem item;
  /// Rust field `stats` serialized as `stats`.
  final Map<String, Object?> stats;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketEntryVersion`.
class MarketEntryVersion {
  const MarketEntryVersion({
    required this.id,
    required this.version,
    required this.formatVer,
    required this.minAppVer,
    required this.maxAppVer,
    required this.changelog,
    required this.projectId,
    required this.runtimePackageId,
    required this.installConfig,
    required this.publisher,
    required this.publishedAt,
  });

  factory MarketEntryVersion.fromJson(Map<String, Object?> json) {
    return MarketEntryVersion(
      id: json['id'] as String,
      version: json['version'] as String,
      formatVer: json['formatVer'] as String,
      minAppVer: json['minAppVer'] as String,
      maxAppVer: json['maxAppVer'] == null ? null : json['maxAppVer'] as String,
      changelog: json['changelog'] == null ? null : json['changelog'] as String,
      projectId: json['projectId'] == null ? null : json['projectId'] as String,
      runtimePackageId: json['runtimePackageId'] == null ? null : json['runtimePackageId'] as String,
      installConfig: json['installConfig'] == null ? null : json['installConfig'] as String,
      publisher: json['publisher'] == null ? null : MarketAuthor.fromJson(json['publisher'] as Map<String, Object?>),
      publishedAt: json['publishedAt'] == null ? null : json['publishedAt'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'version': version,
      'formatVer': formatVer,
      'minAppVer': minAppVer,
      'maxAppVer': maxAppVer,
      'changelog': changelog,
      'projectId': projectId,
      'runtimePackageId': runtimePackageId,
      'installConfig': installConfig,
      'publisher': publisher?.toJson(),
      'publishedAt': publishedAt,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `version` serialized as `version`.
  final String version;
  /// Rust field `format_ver` serialized as `formatVer`.
  final String formatVer;
  /// Rust field `min_app_ver` serialized as `minAppVer`.
  final String minAppVer;
  /// Rust field `max_app_ver` serialized as `maxAppVer`.
  final String? maxAppVer;
  /// Rust field `changelog` serialized as `changelog`.
  final String? changelog;
  /// Rust field `project_id` serialized as `projectId`.
  final String? projectId;
  /// Rust field `runtime_package_id` serialized as `runtimePackageId`.
  final String? runtimePackageId;
  /// Rust field `install_config` serialized as `installConfig`.
  final String? installConfig;
  /// Rust field `publisher` serialized as `publisher`.
  final MarketAuthor? publisher;
  /// Rust field `published_at` serialized as `publishedAt`.
  final String? publishedAt;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketFormatInfo`.
class MarketFormatInfo {
  const MarketFormatInfo({
    required this.kind,
    required this.version,
  });

  factory MarketFormatInfo.fromJson(Map<String, Object?> json) {
    return MarketFormatInfo(
      kind: json['kind'] as String,
      version: json['version'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'kind': kind,
      'version': version,
    };
  }

  /// Rust field `kind` serialized as `kind`.
  final String kind;
  /// Rust field `version` serialized as `version`.
  final String version;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketListPage`.
class MarketListPage {
  const MarketListPage({
    required this.generatedAt,
    required this.sort,
    required this.page,
    required this.pageSize,
    required this.total,
    required this.items,
  });

  factory MarketListPage.fromJson(Map<String, Object?> json) {
    return MarketListPage(
      generatedAt: json['generatedAt'] == null ? null : json['generatedAt'] as String,
      sort: json['sort'] as String,
      page: json['page'] as int,
      pageSize: json['pageSize'] as int,
      total: json['total'] as int,
      items: (json['items'] as List<Object?>).map((item) => MarketEntrySummary.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'generatedAt': generatedAt,
      'sort': sort,
      'page': page,
      'pageSize': pageSize,
      'total': total,
      'items': items.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `generated_at` serialized as `generatedAt`.
  final String? generatedAt;
  /// Rust field `sort` serialized as `sort`.
  final String sort;
  /// Rust field `page` serialized as `page`.
  final int page;
  /// Rust field `page_size` serialized as `pageSize`.
  final int pageSize;
  /// Rust field `total` serialized as `total`.
  final int total;
  /// Rust field `items` serialized as `items`.
  final List<MarketEntrySummary> items;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketManifest`.
class MarketManifest {
  const MarketManifest({
    required this.ok,
    required this.marketVersion,
    required this.generatedAt,
    required this.types,
    required this.formatVersions,
    required this.categories,
    required this.states,
  });

  factory MarketManifest.fromJson(Map<String, Object?> json) {
    return MarketManifest(
      ok: json['ok'] as bool,
      marketVersion: json['marketVersion'] as int,
      generatedAt: json['generatedAt'] == null ? null : json['generatedAt'] as String,
      types: (json['types'] as List<Object?>).map((item) => MarketTypeInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
      formatVersions: (json['formatVersions'] as List<Object?>).map((item) => MarketFormatInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
      categories: (json['categories'] as List<Object?>).map((item) => MarketCategoryInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
      states: (json['states'] as List<Object?>).map((item) => MarketStateInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'ok': ok,
      'marketVersion': marketVersion,
      'generatedAt': generatedAt,
      'types': types.map((item) => item.toJson()).toList(growable: false),
      'formatVersions': formatVersions.map((item) => item.toJson()).toList(growable: false),
      'categories': categories.map((item) => item.toJson()).toList(growable: false),
      'states': states.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `ok` serialized as `ok`.
  final bool ok;
  /// Rust field `market_version` serialized as `marketVersion`.
  final int marketVersion;
  /// Rust field `generated_at` serialized as `generatedAt`.
  final String? generatedAt;
  /// Rust field `types` serialized as `types`.
  final List<MarketTypeInfo> types;
  /// Rust field `format_versions` serialized as `formatVersions`.
  final List<MarketFormatInfo> formatVersions;
  /// Rust field `categories` serialized as `categories`.
  final List<MarketCategoryInfo> categories;
  /// Rust field `states` serialized as `states`.
  final List<MarketStateInfo> states;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketMyEntriesResponse`.
class MarketMyEntriesResponse {
  const MarketMyEntriesResponse({
    required this.ok,
    required this.entries,
    required this.generatedAt,
  });

  factory MarketMyEntriesResponse.fromJson(Map<String, Object?> json) {
    return MarketMyEntriesResponse(
      ok: json['ok'] as bool,
      entries: (json['entries'] as List<Object?>).map((item) => MarketPublisherEntrySummary.fromJson(item as Map<String, Object?>)).toList(growable: false),
      generatedAt: json['generatedAt'] == null ? null : json['generatedAt'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'ok': ok,
      'entries': entries.map((item) => item.toJson()).toList(growable: false),
      'generatedAt': generatedAt,
    };
  }

  /// Rust field `ok` serialized as `ok`.
  final bool ok;
  /// Rust field `entries` serialized as `entries`.
  final List<MarketPublisherEntrySummary> entries;
  /// Rust field `generated_at` serialized as `generatedAt`.
  final String? generatedAt;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketNotification`.
class MarketNotification {
  const MarketNotification({
    required this.id,
    required this.kind,
    required this.entryId,
    required this.commentId,
    required this.actorId,
    required this.title,
    required this.body,
    required this.createdAt,
  });

  factory MarketNotification.fromJson(Map<String, Object?> json) {
    return MarketNotification(
      id: json['id'] as String,
      kind: json['kind'] as String,
      entryId: json['entryId'] == null ? null : json['entryId'] as String,
      commentId: json['commentId'] == null ? null : json['commentId'] as String,
      actorId: json['actorId'] as String,
      title: json['title'] as String,
      body: json['body'] as String,
      createdAt: json['createdAt'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'kind': kind,
      'entryId': entryId,
      'commentId': commentId,
      'actorId': actorId,
      'title': title,
      'body': body,
      'createdAt': createdAt,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `kind` serialized as `kind`.
  final String kind;
  /// Rust field `entry_id` serialized as `entryId`.
  final String? entryId;
  /// Rust field `comment_id` serialized as `commentId`.
  final String? commentId;
  /// Rust field `actor_id` serialized as `actorId`.
  final String actorId;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `body` serialized as `body`.
  final String body;
  /// Rust field `created_at` serialized as `createdAt`.
  final String createdAt;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketNotificationsResponse`.
class MarketNotificationsResponse {
  const MarketNotificationsResponse({
    required this.ok,
    required this.items,
  });

  factory MarketNotificationsResponse.fromJson(Map<String, Object?> json) {
    return MarketNotificationsResponse(
      ok: json['ok'] as bool,
      items: (json['items'] as List<Object?>).map((item) => MarketNotification.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'ok': ok,
      'items': items.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `ok` serialized as `ok`.
  final bool ok;
  /// Rust field `items` serialized as `items`.
  final List<MarketNotification> items;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketPublishResponse`.
class MarketPublishResponse {
  const MarketPublishResponse({
    required this.ok,
    required this.entryId,
    required this.versionId,
  });

  factory MarketPublishResponse.fromJson(Map<String, Object?> json) {
    return MarketPublishResponse(
      ok: json['ok'] as bool,
      entryId: json['entryId'] as String,
      versionId: json['versionId'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'ok': ok,
      'entryId': entryId,
      'versionId': versionId,
    };
  }

  /// Rust field `ok` serialized as `ok`.
  final bool ok;
  /// Rust field `entry_id` serialized as `entryId`.
  final String entryId;
  /// Rust field `version_id` serialized as `versionId`.
  final String versionId;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketPublisherEntrySummary`.
class MarketPublisherEntrySummary {
  const MarketPublisherEntrySummary({
    required this.id,
    required this.title,
    required this.type,
    required this.relation,
    required this.stateCode,
    required this.listingState,
    required this.categoryId,
    required this.updatedAt,
    required this.reasonCodes,
    required this.reviewDetail,
    required this.reviewDetailUpdatedAt,
    required this.revisionAvailableAt,
  });

  factory MarketPublisherEntrySummary.fromJson(Map<String, Object?> json) {
    return MarketPublisherEntrySummary(
      id: json['id'] as String,
      title: json['title'] as String,
      type: json['type'] as String,
      relation: json['relation'] as String,
      stateCode: json['stateCode'] as String,
      listingState: json['listingState'] == null ? null : json['listingState'] as String,
      categoryId: json['categoryId'] == null ? null : json['categoryId'] as String,
      updatedAt: json['updatedAt'] as String,
      reasonCodes: (json['reasonCodes'] as List<Object?>).map((item) => item as String).toList(growable: false),
      reviewDetail: json['reviewDetail'] == null ? null : json['reviewDetail'] as String,
      reviewDetailUpdatedAt: json['reviewDetailUpdatedAt'] == null ? null : json['reviewDetailUpdatedAt'] as String,
      revisionAvailableAt: json['revisionAvailableAt'] == null ? null : json['revisionAvailableAt'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'title': title,
      'type': type,
      'relation': relation,
      'stateCode': stateCode,
      'listingState': listingState,
      'categoryId': categoryId,
      'updatedAt': updatedAt,
      'reasonCodes': reasonCodes.map((item) => item).toList(growable: false),
      'reviewDetail': reviewDetail,
      'reviewDetailUpdatedAt': reviewDetailUpdatedAt,
      'revisionAvailableAt': revisionAvailableAt,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `r#type` serialized as `type`.
  final String type;
  /// Rust field `relation` serialized as `relation`.
  final String relation;
  /// Rust field `state_code` serialized as `stateCode`.
  final String stateCode;
  /// Rust field `listing_state` serialized as `listingState`.
  final String? listingState;
  /// Rust field `category_id` serialized as `categoryId`.
  final String? categoryId;
  /// Rust field `updated_at` serialized as `updatedAt`.
  final String updatedAt;
  /// Rust field `reason_codes` serialized as `reasonCodes`.
  final List<String> reasonCodes;
  /// Rust field `review_detail` serialized as `reviewDetail`.
  final String? reviewDetail;
  /// Rust field `review_detail_updated_at` serialized as `reviewDetailUpdatedAt`.
  final String? reviewDetailUpdatedAt;
  /// Rust field `revision_available_at` serialized as `revisionAvailableAt`.
  final String? revisionAvailableAt;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketReactionCount`.
class MarketReactionCount {
  const MarketReactionCount({
    required this.reaction,
    required this.total,
  });

  factory MarketReactionCount.fromJson(Map<String, Object?> json) {
    return MarketReactionCount(
      reaction: json['reaction'] as String,
      total: json['total'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'reaction': reaction,
      'total': total,
    };
  }

  /// Rust field `reaction` serialized as `reaction`.
  final String reaction;
  /// Rust field `total` serialized as `total`.
  final int total;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketRepoVersion`.
class MarketRepoVersion {
  const MarketRepoVersion({
    required this.refType,
    required this.refName,
    required this.installConfig,
  });

  factory MarketRepoVersion.fromJson(Map<String, Object?> json) {
    return MarketRepoVersion(
      refType: json['refType'] as String,
      refName: json['refName'] as String,
      installConfig: json['installConfig'] == null ? null : json['installConfig'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'refType': refType,
      'refName': refName,
      'installConfig': installConfig,
    };
  }

  /// Rust field `ref_type` serialized as `refType`.
  final String refType;
  /// Rust field `ref_name` serialized as `refName`.
  final String refName;
  /// Rust field `install_config` serialized as `installConfig`.
  final String? installConfig;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketSource`.
class MarketSource {
  const MarketSource({
    required this.kind,
    required this.url,
  });

  factory MarketSource.fromJson(Map<String, Object?> json) {
    return MarketSource(
      kind: json['kind'] as String,
      url: json['url'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'kind': kind,
      'url': url,
    };
  }

  /// Rust field `kind` serialized as `kind`.
  final String kind;
  /// Rust field `url` serialized as `url`.
  final String url;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketStateInfo`.
class MarketStateInfo {
  const MarketStateInfo({
    required this.code,
    required this.label,
  });

  factory MarketStateInfo.fromJson(Map<String, Object?> json) {
    return MarketStateInfo(
      code: json['code'] as String,
      label: json['label'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'code': code,
      'label': label,
    };
  }

  /// Rust field `code` serialized as `code`.
  final String code;
  /// Rust field `label` serialized as `label`.
  final String label;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketStatsEntryResponse`.
class MarketStatsEntryResponse {
  const MarketStatsEntryResponse({
    required this.downloads,
    required this.lastDownloadAt,
    required this.updatedAt,
  });

  factory MarketStatsEntryResponse.fromJson(Map<String, Object?> json) {
    return MarketStatsEntryResponse(
      downloads: json['downloads'] as int,
      lastDownloadAt: json['lastDownloadAt'] == null ? null : json['lastDownloadAt'] as String,
      updatedAt: json['updatedAt'] == null ? null : json['updatedAt'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'downloads': downloads,
      'lastDownloadAt': lastDownloadAt,
      'updatedAt': updatedAt,
    };
  }

  /// Rust field `downloads` serialized as `downloads`.
  final int downloads;
  /// Rust field `last_download_at` serialized as `lastDownloadAt`.
  final String? lastDownloadAt;
  /// Rust field `updated_at` serialized as `updatedAt`.
  final String? updatedAt;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketTypeInfo`.
class MarketTypeInfo {
  const MarketTypeInfo({
    required this.id,
    required this.name,
    required this.description,
  });

  factory MarketTypeInfo.fromJson(Map<String, Object?> json) {
    return MarketTypeInfo(
      id: json['id'] as String,
      name: json['name'] as String,
      description: json['description'] == null ? null : json['description'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'name': name,
      'description': description,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final String? description;
}

/// Generated Dart model for Rust type `operit_providers::market::MarketStatsApiService::MarketTypeStatsResponse`.
class MarketTypeStatsResponse {
  const MarketTypeStatsResponse({
    required this.updatedAt,
    required this.items,
  });

  factory MarketTypeStatsResponse.fromJson(Map<String, Object?> json) {
    return MarketTypeStatsResponse(
      updatedAt: json['updatedAt'] == null ? null : json['updatedAt'] as String,
      items: (json['items'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, MarketStatsEntryResponse.fromJson(value as Map<String, Object?>))),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'updatedAt': updatedAt,
      'items': items.map((key, value) => MapEntry(key, value.toJson())),
    };
  }

  /// Rust field `updated_at` serialized as `updatedAt`.
  final String? updatedAt;
  /// Rust field `items` serialized as `items`.
  final Map<String, MarketStatsEntryResponse> items;
}

/// Generated Dart model for Rust type `operit_runtime::core::events::RuntimeEvent::RuntimeEvent`.
class RuntimeEvent {
  const RuntimeEvent({
    required this.domain,
    required this.source,
    required this.topic,
    required this.platform,
    required this.payload,
    required this.occurredAtMillis,
  });

  factory RuntimeEvent.fromJson(Map<String, Object?> json) {
    return RuntimeEvent(
      domain: RuntimeEventDomain.fromJson(json['domain']),
      source: RuntimeEventSource.fromJson(json['source']),
      topic: RuntimeEventTopic.fromJson(json['topic']),
      platform: RuntimeEventPlatform.fromJson(json['platform']),
      payload: json['payload'],
      occurredAtMillis: json['occurredAtMillis'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'domain': domain.toJson(),
      'source': source.toJson(),
      'topic': topic.toJson(),
      'platform': platform.toJson(),
      'payload': payload,
      'occurredAtMillis': occurredAtMillis,
    };
  }

  /// Rust field `domain` serialized as `domain`.
  final RuntimeEventDomain domain;
  /// Rust field `source` serialized as `source`.
  final RuntimeEventSource source;
  /// Rust field `topic` serialized as `topic`.
  final RuntimeEventTopic topic;
  /// Rust field `platform` serialized as `platform`.
  final RuntimeEventPlatform platform;
  /// Rust field `payload` serialized as `payload`.
  final Object? payload;
  /// Rust field `occurredAtMillis` serialized as `occurredAtMillis`.
  final int occurredAtMillis;
}

/// Generated Dart enum for Rust type `operit_runtime::core::events::RuntimeEvent::RuntimeEventDomain`.
enum RuntimeEventDomain {
  app('app'),
  host('host'),
  runtime('runtime'),
  ;

  const RuntimeEventDomain(this.value);

  final String value;

  factory RuntimeEventDomain.fromJson(Object? value) {
    return switch (value) {
      'app' => RuntimeEventDomain.app,
      'host' => RuntimeEventDomain.host,
      'runtime' => RuntimeEventDomain.runtime,
      _ => throw ArgumentError('Unknown RuntimeEventDomain: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_runtime::core::events::RuntimeEvent::RuntimeEventPlatform`.
enum RuntimeEventPlatform {
  android('android'),
  linux('linux'),
  windows('windows'),
  macos('macos'),
  ios('ios'),
  ohos('ohos'),
  web('web'),
  ;

  const RuntimeEventPlatform(this.value);

  final String value;

  factory RuntimeEventPlatform.fromJson(Object? value) {
    return switch (value) {
      'android' => RuntimeEventPlatform.android,
      'linux' => RuntimeEventPlatform.linux,
      'windows' => RuntimeEventPlatform.windows,
      'macos' => RuntimeEventPlatform.macos,
      'ios' => RuntimeEventPlatform.ios,
      'ohos' => RuntimeEventPlatform.ohos,
      'web' => RuntimeEventPlatform.web,
      _ => throw ArgumentError('Unknown RuntimeEventPlatform: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_runtime::core::events::RuntimeEvent::RuntimeEventSource`.
enum RuntimeEventSource {
  androidBroadcast('android.broadcast'),
  androidLifecycle('android.lifecycle'),
  linuxDbus('linux.dbus'),
  windowsSystem('windows.system'),
  macosSystem('macos.system'),
  iosSystem('ios.system'),
  ohosSystem('ohos.system'),
  webEvent('web.event'),
  flutterLifecycle('flutter.lifecycle'),
  runtimeTimer('runtime.timer'),
  runtimeInterval('runtime.interval'),
  ;

  const RuntimeEventSource(this.value);

  final String value;

  factory RuntimeEventSource.fromJson(Object? value) {
    return switch (value) {
      'android.broadcast' => RuntimeEventSource.androidBroadcast,
      'android.lifecycle' => RuntimeEventSource.androidLifecycle,
      'linux.dbus' => RuntimeEventSource.linuxDbus,
      'windows.system' => RuntimeEventSource.windowsSystem,
      'macos.system' => RuntimeEventSource.macosSystem,
      'ios.system' => RuntimeEventSource.iosSystem,
      'ohos.system' => RuntimeEventSource.ohosSystem,
      'web.event' => RuntimeEventSource.webEvent,
      'flutter.lifecycle' => RuntimeEventSource.flutterLifecycle,
      'runtime.timer' => RuntimeEventSource.runtimeTimer,
      'runtime.interval' => RuntimeEventSource.runtimeInterval,
      _ => throw ArgumentError('Unknown RuntimeEventSource: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_runtime::core::events::RuntimeEvent::RuntimeEventTopic`.
enum RuntimeEventTopic {
  appLifecycleResumed('app.lifecycle.resumed'),
  appLifecycleInactive('app.lifecycle.inactive'),
  appLifecyclePaused('app.lifecycle.paused'),
  appLifecycleDetached('app.lifecycle.detached'),
  appLifecycleHidden('app.lifecycle.hidden'),
  appLifecycleLowMemory('app.lifecycle.low_memory'),
  appLifecycleTrimMemory('app.lifecycle.trim_memory'),
  activityLifecycleCreate('activity.lifecycle.create'),
  activityLifecycleStart('activity.lifecycle.start'),
  activityLifecycleResume('activity.lifecycle.resume'),
  activityLifecyclePause('activity.lifecycle.pause'),
  activityLifecycleStop('activity.lifecycle.stop'),
  activityLifecycleDestroy('activity.lifecycle.destroy'),
  systemBootCompleted('system.boot.completed'),
  systemPowerConnected('system.power.connected'),
  systemPowerDisconnected('system.power.disconnected'),
  systemPowerSleep('system.power.sleep'),
  systemPowerWake('system.power.wake'),
  systemBatteryLow('system.battery.low'),
  systemBatteryOkay('system.battery.okay'),
  systemScreenOn('system.screen.on'),
  systemScreenOff('system.screen.off'),
  systemUserPresent('system.user.present'),
  systemTimeTick('system.time.tick'),
  systemDateChanged('system.date.changed'),
  systemTimezoneChanged('system.timezone.changed'),
  systemAirplaneModeChanged('system.airplane_mode.changed'),
  systemHeadsetPlug('system.headset.plug'),
  systemNetworkChanged('system.network.changed'),
  systemSessionLock('system.session.lock'),
  systemSessionUnlock('system.session.unlock'),
  bluetoothDeviceFound('bluetooth.device.found'),
  bluetoothDeviceNameChanged('bluetooth.device.name_changed'),
  bluetoothDeviceConnected('bluetooth.device.connected'),
  bluetoothDeviceDisconnected('bluetooth.device.disconnected'),
  bluetoothDeviceBondStateChanged('bluetooth.device.bond_state_changed'),
  bluetoothAdapterConnectionStateChanged('bluetooth.adapter.connection_state_changed'),
  bluetoothAdapterPoweredChanged('bluetooth.adapter.powered_changed'),
  ;

  const RuntimeEventTopic(this.value);

  final String value;

  factory RuntimeEventTopic.fromJson(Object? value) {
    return switch (value) {
      'app.lifecycle.resumed' => RuntimeEventTopic.appLifecycleResumed,
      'app.lifecycle.inactive' => RuntimeEventTopic.appLifecycleInactive,
      'app.lifecycle.paused' => RuntimeEventTopic.appLifecyclePaused,
      'app.lifecycle.detached' => RuntimeEventTopic.appLifecycleDetached,
      'app.lifecycle.hidden' => RuntimeEventTopic.appLifecycleHidden,
      'app.lifecycle.low_memory' => RuntimeEventTopic.appLifecycleLowMemory,
      'app.lifecycle.trim_memory' => RuntimeEventTopic.appLifecycleTrimMemory,
      'activity.lifecycle.create' => RuntimeEventTopic.activityLifecycleCreate,
      'activity.lifecycle.start' => RuntimeEventTopic.activityLifecycleStart,
      'activity.lifecycle.resume' => RuntimeEventTopic.activityLifecycleResume,
      'activity.lifecycle.pause' => RuntimeEventTopic.activityLifecyclePause,
      'activity.lifecycle.stop' => RuntimeEventTopic.activityLifecycleStop,
      'activity.lifecycle.destroy' => RuntimeEventTopic.activityLifecycleDestroy,
      'system.boot.completed' => RuntimeEventTopic.systemBootCompleted,
      'system.power.connected' => RuntimeEventTopic.systemPowerConnected,
      'system.power.disconnected' => RuntimeEventTopic.systemPowerDisconnected,
      'system.power.sleep' => RuntimeEventTopic.systemPowerSleep,
      'system.power.wake' => RuntimeEventTopic.systemPowerWake,
      'system.battery.low' => RuntimeEventTopic.systemBatteryLow,
      'system.battery.okay' => RuntimeEventTopic.systemBatteryOkay,
      'system.screen.on' => RuntimeEventTopic.systemScreenOn,
      'system.screen.off' => RuntimeEventTopic.systemScreenOff,
      'system.user.present' => RuntimeEventTopic.systemUserPresent,
      'system.time.tick' => RuntimeEventTopic.systemTimeTick,
      'system.date.changed' => RuntimeEventTopic.systemDateChanged,
      'system.timezone.changed' => RuntimeEventTopic.systemTimezoneChanged,
      'system.airplane_mode.changed' => RuntimeEventTopic.systemAirplaneModeChanged,
      'system.headset.plug' => RuntimeEventTopic.systemHeadsetPlug,
      'system.network.changed' => RuntimeEventTopic.systemNetworkChanged,
      'system.session.lock' => RuntimeEventTopic.systemSessionLock,
      'system.session.unlock' => RuntimeEventTopic.systemSessionUnlock,
      'bluetooth.device.found' => RuntimeEventTopic.bluetoothDeviceFound,
      'bluetooth.device.name_changed' => RuntimeEventTopic.bluetoothDeviceNameChanged,
      'bluetooth.device.connected' => RuntimeEventTopic.bluetoothDeviceConnected,
      'bluetooth.device.disconnected' => RuntimeEventTopic.bluetoothDeviceDisconnected,
      'bluetooth.device.bond_state_changed' => RuntimeEventTopic.bluetoothDeviceBondStateChanged,
      'bluetooth.adapter.connection_state_changed' => RuntimeEventTopic.bluetoothAdapterConnectionStateChanged,
      'bluetooth.adapter.powered_changed' => RuntimeEventTopic.bluetoothAdapterPoweredChanged,
      _ => throw ArgumentError('Unknown RuntimeEventTopic: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_runtime::data::backup::Operit1SnapshotImportManager::Operit1DataStoreFilePreview`.
class Operit1DataStoreFilePreview {
  const Operit1DataStoreFilePreview({
    required this.fileName,
    required this.keyCount,
  });

  factory Operit1DataStoreFilePreview.fromJson(Map<String, Object?> json) {
    return Operit1DataStoreFilePreview(
      fileName: json['fileName'] as String,
      keyCount: json['keyCount'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'fileName': fileName,
      'keyCount': keyCount,
    };
  }

  /// Rust field `fileName` serialized as `fileName`.
  final String fileName;
  /// Rust field `keyCount` serialized as `keyCount`.
  final int keyCount;
}

/// Generated Dart model for Rust type `operit_runtime::data::backup::Operit1SnapshotImportManager::Operit1ModelConfigImportResult`.
class Operit1ModelConfigImportResult {
  const Operit1ModelConfigImportResult({
    required this.providerId,
    required this.providerTypeId,
    required this.providerName,
    required this.modelId,
    required this.importedModelCount,
    required this.chatBindingUpdated,
    required this.skippedFields,
  });

  factory Operit1ModelConfigImportResult.fromJson(Map<String, Object?> json) {
    return Operit1ModelConfigImportResult(
      providerId: json['providerId'] as String,
      providerTypeId: json['providerTypeId'] as String,
      providerName: json['providerName'] as String,
      modelId: json['modelId'] as String,
      importedModelCount: json['importedModelCount'] as int,
      chatBindingUpdated: json['chatBindingUpdated'] as bool,
      skippedFields: (json['skippedFields'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'providerId': providerId,
      'providerTypeId': providerTypeId,
      'providerName': providerName,
      'modelId': modelId,
      'importedModelCount': importedModelCount,
      'chatBindingUpdated': chatBindingUpdated,
      'skippedFields': skippedFields.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `providerId` serialized as `providerId`.
  final String providerId;
  /// Rust field `providerTypeId` serialized as `providerTypeId`.
  final String providerTypeId;
  /// Rust field `providerName` serialized as `providerName`.
  final String providerName;
  /// Rust field `modelId` serialized as `modelId`.
  final String modelId;
  /// Rust field `importedModelCount` serialized as `importedModelCount`.
  final int importedModelCount;
  /// Rust field `chatBindingUpdated` serialized as `chatBindingUpdated`.
  final bool chatBindingUpdated;
  /// Rust field `skippedFields` serialized as `skippedFields`.
  final List<String> skippedFields;
}

/// Generated Dart model for Rust type `operit_runtime::data::backup::Operit1SnapshotImportManager::Operit1ModelConfigPreview`.
class Operit1ModelConfigPreview {
  const Operit1ModelConfigPreview({
    required this.configId,
    required this.name,
    required this.providerTypeId,
    required this.providerDisplayName,
    required this.endpoint,
    required this.modelIds,
    required this.selectedModelId,
    required this.selectedModelIndex,
  });

  factory Operit1ModelConfigPreview.fromJson(Map<String, Object?> json) {
    return Operit1ModelConfigPreview(
      configId: json['configId'] as String,
      name: json['name'] as String,
      providerTypeId: json['providerTypeId'] as String,
      providerDisplayName: json['providerDisplayName'] as String,
      endpoint: json['endpoint'] as String,
      modelIds: (json['modelIds'] as List<Object?>).map((item) => item as String).toList(growable: false),
      selectedModelId: json['selectedModelId'] == null ? null : json['selectedModelId'] as String,
      selectedModelIndex: json['selectedModelIndex'] == null ? null : json['selectedModelIndex'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'configId': configId,
      'name': name,
      'providerTypeId': providerTypeId,
      'providerDisplayName': providerDisplayName,
      'endpoint': endpoint,
      'modelIds': modelIds.map((item) => item).toList(growable: false),
      'selectedModelId': selectedModelId,
      'selectedModelIndex': selectedModelIndex,
    };
  }

  /// Rust field `configId` serialized as `configId`.
  final String configId;
  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `providerTypeId` serialized as `providerTypeId`.
  final String providerTypeId;
  /// Rust field `providerDisplayName` serialized as `providerDisplayName`.
  final String providerDisplayName;
  /// Rust field `endpoint` serialized as `endpoint`.
  final String endpoint;
  /// Rust field `modelIds` serialized as `modelIds`.
  final List<String> modelIds;
  /// Rust field `selectedModelId` serialized as `selectedModelId`.
  final String? selectedModelId;
  /// Rust field `selectedModelIndex` serialized as `selectedModelIndex`.
  final int? selectedModelIndex;
}

/// Generated Dart model for Rust type `operit_runtime::data::backup::Operit1SnapshotImportManager::Operit1ModelConfigSnapshotPreview`.
class Operit1ModelConfigSnapshotPreview {
  const Operit1ModelConfigSnapshotPreview({
    required this.formatVersion,
    required this.packageName,
    required this.createdAt,
    required this.configs,
    required this.chatConfigId,
    required this.chatModelId,
    required this.chatModelIndex,
  });

  factory Operit1ModelConfigSnapshotPreview.fromJson(Map<String, Object?> json) {
    return Operit1ModelConfigSnapshotPreview(
      formatVersion: json['formatVersion'] as int,
      packageName: json['packageName'] as String,
      createdAt: json['createdAt'] as int,
      configs: (json['configs'] as List<Object?>).map((item) => Operit1ModelConfigPreview.fromJson(item as Map<String, Object?>)).toList(growable: false),
      chatConfigId: json['chatConfigId'] == null ? null : json['chatConfigId'] as String,
      chatModelId: json['chatModelId'] == null ? null : json['chatModelId'] as String,
      chatModelIndex: json['chatModelIndex'] == null ? null : json['chatModelIndex'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'formatVersion': formatVersion,
      'packageName': packageName,
      'createdAt': createdAt,
      'configs': configs.map((item) => item.toJson()).toList(growable: false),
      'chatConfigId': chatConfigId,
      'chatModelId': chatModelId,
      'chatModelIndex': chatModelIndex,
    };
  }

  /// Rust field `formatVersion` serialized as `formatVersion`.
  final int formatVersion;
  /// Rust field `packageName` serialized as `packageName`.
  final String packageName;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
  /// Rust field `configs` serialized as `configs`.
  final List<Operit1ModelConfigPreview> configs;
  /// Rust field `chatConfigId` serialized as `chatConfigId`.
  final String? chatConfigId;
  /// Rust field `chatModelId` serialized as `chatModelId`.
  final String? chatModelId;
  /// Rust field `chatModelIndex` serialized as `chatModelIndex`.
  final int? chatModelIndex;
}

/// Generated Dart model for Rust type `operit_runtime::data::backup::Operit1SnapshotImportManager::Operit1SnapshotImportProgress`.
class Operit1SnapshotImportProgress {
  const Operit1SnapshotImportProgress({
    required this.stage,
    required this.title,
    required this.detail,
    required this.progress,
    required this.active,
  });

  factory Operit1SnapshotImportProgress.fromJson(Map<String, Object?> json) {
    return Operit1SnapshotImportProgress(
      stage: json['stage'] as String,
      title: json['title'] as String,
      detail: json['detail'] as String,
      progress: (json['progress'] as num).toDouble(),
      active: json['active'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'stage': stage,
      'title': title,
      'detail': detail,
      'progress': progress,
      'active': active,
    };
  }

  /// Rust field `stage` serialized as `stage`.
  final String stage;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `detail` serialized as `detail`.
  final String detail;
  /// Rust field `progress` serialized as `progress`.
  final double progress;
  /// Rust field `active` serialized as `active`.
  final bool active;
}

/// Generated Dart model for Rust type `operit_runtime::data::backup::Operit1SnapshotImportManager::Operit1SnapshotImportResult`.
class Operit1SnapshotImportResult {
  const Operit1SnapshotImportResult({
    required this.modelConfig,
    required this.importedDatastoreFiles,
    required this.importedDatastoreKeys,
    required this.importedChats,
    required this.importedMessages,
    required this.importedMemories,
    required this.importedMemoryLinks,
    required this.importedFiles,
    required this.importedExternalFiles,
    required this.importedWorkspaces,
    required this.importedWorkspaceFiles,
  });

  factory Operit1SnapshotImportResult.fromJson(Map<String, Object?> json) {
    return Operit1SnapshotImportResult(
      modelConfig: Operit1ModelConfigImportResult.fromJson(json['modelConfig'] as Map<String, Object?>),
      importedDatastoreFiles: json['importedDatastoreFiles'] as int,
      importedDatastoreKeys: json['importedDatastoreKeys'] as int,
      importedChats: json['importedChats'] as int,
      importedMessages: json['importedMessages'] as int,
      importedMemories: json['importedMemories'] as int,
      importedMemoryLinks: json['importedMemoryLinks'] as int,
      importedFiles: json['importedFiles'] as int,
      importedExternalFiles: json['importedExternalFiles'] as int,
      importedWorkspaces: json['importedWorkspaces'] as int,
      importedWorkspaceFiles: json['importedWorkspaceFiles'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'modelConfig': modelConfig.toJson(),
      'importedDatastoreFiles': importedDatastoreFiles,
      'importedDatastoreKeys': importedDatastoreKeys,
      'importedChats': importedChats,
      'importedMessages': importedMessages,
      'importedMemories': importedMemories,
      'importedMemoryLinks': importedMemoryLinks,
      'importedFiles': importedFiles,
      'importedExternalFiles': importedExternalFiles,
      'importedWorkspaces': importedWorkspaces,
      'importedWorkspaceFiles': importedWorkspaceFiles,
    };
  }

  /// Rust field `modelConfig` serialized as `modelConfig`.
  final Operit1ModelConfigImportResult modelConfig;
  /// Rust field `importedDatastoreFiles` serialized as `importedDatastoreFiles`.
  final int importedDatastoreFiles;
  /// Rust field `importedDatastoreKeys` serialized as `importedDatastoreKeys`.
  final int importedDatastoreKeys;
  /// Rust field `importedChats` serialized as `importedChats`.
  final int importedChats;
  /// Rust field `importedMessages` serialized as `importedMessages`.
  final int importedMessages;
  /// Rust field `importedMemories` serialized as `importedMemories`.
  final int importedMemories;
  /// Rust field `importedMemoryLinks` serialized as `importedMemoryLinks`.
  final int importedMemoryLinks;
  /// Rust field `importedFiles` serialized as `importedFiles`.
  final int importedFiles;
  /// Rust field `importedExternalFiles` serialized as `importedExternalFiles`.
  final int importedExternalFiles;
  /// Rust field `importedWorkspaces` serialized as `importedWorkspaces`.
  final int importedWorkspaces;
  /// Rust field `importedWorkspaceFiles` serialized as `importedWorkspaceFiles`.
  final int importedWorkspaceFiles;
}

/// Generated Dart model for Rust type `operit_runtime::data::backup::Operit1SnapshotImportManager::Operit1SnapshotPreview`.
class Operit1SnapshotPreview {
  const Operit1SnapshotPreview({
    required this.formatVersion,
    required this.packageName,
    required this.createdAt,
    required this.modelConfig,
    required this.datastoreFiles,
    required this.chatCount,
    required this.messageCount,
    required this.importedFileCount,
    required this.importedExternalFileCount,
    required this.detectedDomains,
  });

  factory Operit1SnapshotPreview.fromJson(Map<String, Object?> json) {
    return Operit1SnapshotPreview(
      formatVersion: json['formatVersion'] as int,
      packageName: json['packageName'] as String,
      createdAt: json['createdAt'] as int,
      modelConfig: Operit1ModelConfigSnapshotPreview.fromJson(json['modelConfig'] as Map<String, Object?>),
      datastoreFiles: (json['datastoreFiles'] as List<Object?>).map((item) => Operit1DataStoreFilePreview.fromJson(item as Map<String, Object?>)).toList(growable: false),
      chatCount: json['chatCount'] as int,
      messageCount: json['messageCount'] as int,
      importedFileCount: json['importedFileCount'] as int,
      importedExternalFileCount: json['importedExternalFileCount'] as int,
      detectedDomains: (json['detectedDomains'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'formatVersion': formatVersion,
      'packageName': packageName,
      'createdAt': createdAt,
      'modelConfig': modelConfig.toJson(),
      'datastoreFiles': datastoreFiles.map((item) => item.toJson()).toList(growable: false),
      'chatCount': chatCount,
      'messageCount': messageCount,
      'importedFileCount': importedFileCount,
      'importedExternalFileCount': importedExternalFileCount,
      'detectedDomains': detectedDomains.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `formatVersion` serialized as `formatVersion`.
  final int formatVersion;
  /// Rust field `packageName` serialized as `packageName`.
  final String packageName;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
  /// Rust field `modelConfig` serialized as `modelConfig`.
  final Operit1ModelConfigSnapshotPreview modelConfig;
  /// Rust field `datastoreFiles` serialized as `datastoreFiles`.
  final List<Operit1DataStoreFilePreview> datastoreFiles;
  /// Rust field `chatCount` serialized as `chatCount`.
  final int chatCount;
  /// Rust field `messageCount` serialized as `messageCount`.
  final int messageCount;
  /// Rust field `importedFileCount` serialized as `importedFileCount`.
  final int importedFileCount;
  /// Rust field `importedExternalFileCount` serialized as `importedExternalFileCount`.
  final int importedExternalFileCount;
  /// Rust field `detectedDomains` serialized as `detectedDomains`.
  final List<String> detectedDomains;
}

/// Generated Dart model for Rust type `operit_runtime::data::backup::RawSnapshotBackupManager::RawSnapshotManifest`.
class RawSnapshotManifest {
  const RawSnapshotManifest({
    required this.formatVersion,
    required this.createdAt,
    required this.includes,
  });

  factory RawSnapshotManifest.fromJson(Map<String, Object?> json) {
    return RawSnapshotManifest(
      formatVersion: json['formatVersion'] as int,
      createdAt: json['createdAt'] as int,
      includes: (json['includes'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'formatVersion': formatVersion,
      'createdAt': createdAt,
      'includes': includes.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `formatVersion` serialized as `formatVersion`.
  final int formatVersion;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
  /// Rust field `includes` serialized as `includes`.
  final List<String> includes;
}

/// Generated Dart model for Rust type `operit_runtime::data::preferences::CharacterCardManager::CharacterCardImportResult`.
class CharacterCardImportResult {
  const CharacterCardImportResult({
    required this.newValue,
    required this.updated,
    required this.skipped,
    required this.total,
  });

  factory CharacterCardImportResult.fromJson(Map<String, Object?> json) {
    return CharacterCardImportResult(
      newValue: json['new'] as int,
      updated: json['updated'] as int,
      skipped: json['skipped'] as int,
      total: json['total'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'new': newValue,
      'updated': updated,
      'skipped': skipped,
      'total': total,
    };
  }

  /// Rust field `new` serialized as `new`.
  final int newValue;
  /// Rust field `updated` serialized as `updated`.
  final int updated;
  /// Rust field `skipped` serialized as `skipped`.
  final int skipped;
  /// Rust field `total` serialized as `total`.
  final int total;
}

/// Generated Dart model for Rust type `operit_runtime::data::preferences::CharacterGroupCardManager::CharacterGroupImportResult`.
class CharacterGroupImportResult {
  const CharacterGroupImportResult({
    required this.newValue,
    required this.updated,
    required this.skipped,
    required this.total,
  });

  factory CharacterGroupImportResult.fromJson(Map<String, Object?> json) {
    return CharacterGroupImportResult(
      newValue: json['new'] as int,
      updated: json['updated'] as int,
      skipped: json['skipped'] as int,
      total: json['total'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'new': newValue,
      'updated': updated,
      'skipped': skipped,
      'total': total,
    };
  }

  /// Rust field `new` serialized as `new`.
  final int newValue;
  /// Rust field `updated` serialized as `updated`.
  final int updated;
  /// Rust field `skipped` serialized as `skipped`.
  final int skipped;
  /// Rust field `total` serialized as `total`.
  final int total;
}

/// Generated Dart model for Rust type `operit_runtime::data::preferences::FunctionalConfigManager::FunctionModelBinding`.
class FunctionModelBinding {
  const FunctionModelBinding({
    required this.providerId,
    required this.modelId,
  });

  factory FunctionModelBinding.fromJson(Map<String, Object?> json) {
    return FunctionModelBinding(
      providerId: json['providerId'] as String,
      modelId: json['modelId'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'providerId': providerId,
      'modelId': modelId,
    };
  }

  /// Rust field `providerId` serialized as `providerId`.
  final String providerId;
  /// Rust field `modelId` serialized as `modelId`.
  final String modelId;
}

/// Generated Dart model for Rust type `operit_runtime::data::preferences::GitHubAuthPreferences::GitHubUser`.
class CoreDataPreferencesGitHubAuthPreferencesGitHubUser {
  const CoreDataPreferencesGitHubAuthPreferencesGitHubUser({
    required this.id,
    required this.login,
    required this.name,
    required this.email,
    required this.avatarUrl,
    required this.bio,
    required this.publicRepos,
    required this.followers,
    required this.following,
  });

  factory CoreDataPreferencesGitHubAuthPreferencesGitHubUser.fromJson(Map<String, Object?> json) {
    return CoreDataPreferencesGitHubAuthPreferencesGitHubUser(
      id: json['id'] as String,
      login: json['login'] as String,
      name: json['name'] == null ? null : json['name'] as String,
      email: json['email'] == null ? null : json['email'] as String,
      avatarUrl: json['avatar_url'] as String,
      bio: json['bio'] == null ? null : json['bio'] as String,
      publicRepos: json['public_repos'] == null ? null : json['public_repos'] as int,
      followers: json['followers'] == null ? null : json['followers'] as int,
      following: json['following'] == null ? null : json['following'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'login': login,
      'name': name,
      'email': email,
      'avatar_url': avatarUrl,
      'bio': bio,
      'public_repos': publicRepos,
      'followers': followers,
      'following': following,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `login` serialized as `login`.
  final String login;
  /// Rust field `name` serialized as `name`.
  final String? name;
  /// Rust field `email` serialized as `email`.
  final String? email;
  /// Rust field `avatarUrl` serialized as `avatar_url`.
  final String avatarUrl;
  /// Rust field `bio` serialized as `bio`.
  final String? bio;
  /// Rust field `publicRepos` serialized as `public_repos`.
  final int? publicRepos;
  /// Rust field `followers` serialized as `followers`.
  final int? followers;
  /// Rust field `following` serialized as `following`.
  final int? following;
}

/// Generated Dart model for Rust type `operit_runtime::plugins::toolpkg::ToolPkgInputMenuToggleBridge::InputMenuToggleDefinitionSnapshot`.
class InputMenuToggleDefinitionSnapshot {
  const InputMenuToggleDefinitionSnapshot({
    required this.id,
    required this.title,
    required this.description,
    required this.icon,
    required this.isChecked,
    required this.isEnabled,
    required this.slot,
  });

  factory InputMenuToggleDefinitionSnapshot.fromJson(Map<String, Object?> json) {
    return InputMenuToggleDefinitionSnapshot(
      id: json['id'] as String,
      title: json['title'] == null ? null : json['title'] as String,
      description: json['description'] == null ? null : json['description'] as String,
      icon: json['icon'] == null ? null : json['icon'] as String,
      isChecked: json['isChecked'] as bool,
      isEnabled: json['isEnabled'] as bool,
      slot: json['slot'] == null ? null : json['slot'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'title': title,
      'description': description,
      'icon': icon,
      'isChecked': isChecked,
      'isEnabled': isEnabled,
      'slot': slot,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `title` serialized as `title`.
  final String? title;
  /// Rust field `description` serialized as `description`.
  final String? description;
  /// Rust field `icon` serialized as `icon`.
  final String? icon;
  /// Rust field `isChecked` serialized as `isChecked`.
  final bool isChecked;
  /// Rust field `isEnabled` serialized as `isEnabled`.
  final bool isEnabled;
  /// Rust field `slot` serialized as `slot`.
  final String? slot;
}

/// Generated Dart model for Rust type `operit_runtime::services::ArchiveTransferManager::StagedArchive`.
class StagedArchive {
  const StagedArchive({
    required this.archiveId,
    required this.byteLength,
  });

  factory StagedArchive.fromJson(Map<String, Object?> json) {
    return StagedArchive(
      archiveId: json['archiveId'] as String,
      byteLength: json['byteLength'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'archiveId': archiveId,
      'byteLength': byteLength,
    };
  }

  /// Rust field `archiveId` serialized as `archiveId`.
  final String archiveId;
  /// Rust field `byteLength` serialized as `byteLength`.
  final int byteLength;
}

/// Generated Dart model for Rust type `operit_runtime::services::GitHubOAuthBrokerService::GitHubOAuthBrokerLoginCompletion`.
class GitHubOAuthBrokerLoginCompletion {
  const GitHubOAuthBrokerLoginCompletion({
    required this.attemptId,
    required this.completionUrl,
  });

  factory GitHubOAuthBrokerLoginCompletion.fromJson(Map<String, Object?> json) {
    return GitHubOAuthBrokerLoginCompletion(
      attemptId: json['attemptId'] as String,
      completionUrl: json['completionUrl'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'attemptId': attemptId,
      'completionUrl': completionUrl,
    };
  }

  /// Rust field `attemptId` serialized as `attemptId`.
  final String attemptId;
  /// Rust field `completionUrl` serialized as `completionUrl`.
  final String completionUrl;
}

/// Generated Dart model for Rust type `operit_runtime::services::GitHubOAuthBrokerService::GitHubOAuthBrokerLoginResult`.
class GitHubOAuthBrokerLoginResult {
  const GitHubOAuthBrokerLoginResult({
    required this.githubId,
    required this.login,
  });

  factory GitHubOAuthBrokerLoginResult.fromJson(Map<String, Object?> json) {
    return GitHubOAuthBrokerLoginResult(
      githubId: json['githubId'] as String,
      login: json['login'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'githubId': githubId,
      'login': login,
    };
  }

  /// Rust field `githubId` serialized as `githubId`.
  final String githubId;
  /// Rust field `login` serialized as `login`.
  final String login;
}

/// Generated Dart model for Rust type `operit_runtime::services::GitHubOAuthBrokerService::GitHubOAuthBrokerLoginStart`.
class GitHubOAuthBrokerLoginStart {
  const GitHubOAuthBrokerLoginStart({
    required this.attemptId,
    required this.authorizationUrl,
    required this.expiresAt,
  });

  factory GitHubOAuthBrokerLoginStart.fromJson(Map<String, Object?> json) {
    return GitHubOAuthBrokerLoginStart(
      attemptId: json['attemptId'] as String,
      authorizationUrl: json['authorizationUrl'] as String,
      expiresAt: json['expiresAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'attemptId': attemptId,
      'authorizationUrl': authorizationUrl,
      'expiresAt': expiresAt,
    };
  }

  /// Rust field `attemptId` serialized as `attemptId`.
  final String attemptId;
  /// Rust field `authorizationUrl` serialized as `authorizationUrl`.
  final String authorizationUrl;
  /// Rust field `expiresAt` serialized as `expiresAt`.
  final int expiresAt;
}

/// Generated Dart model for Rust type `operit_runtime::services::LocalModelService::LocalModelBundleInstallResult`.
class LocalModelBundleInstallResult {
  const LocalModelBundleInstallResult({
    required this.installedModel,
    required this.installedEngine,
    required this.modelDownloadedBytes,
    required this.engineDownloadedBytes,
  });

  factory LocalModelBundleInstallResult.fromJson(Map<String, Object?> json) {
    return LocalModelBundleInstallResult(
      installedModel: InstalledLocalModel.fromJson(json['installedModel'] as Map<String, Object?>),
      installedEngine: InstalledLocalEngine.fromJson(json['installedEngine'] as Map<String, Object?>),
      modelDownloadedBytes: json['modelDownloadedBytes'] as int,
      engineDownloadedBytes: json['engineDownloadedBytes'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'installedModel': installedModel.toJson(),
      'installedEngine': installedEngine.toJson(),
      'modelDownloadedBytes': modelDownloadedBytes,
      'engineDownloadedBytes': engineDownloadedBytes,
    };
  }

  /// Rust field `installedModel` serialized as `installedModel`.
  final InstalledLocalModel installedModel;
  /// Rust field `installedEngine` serialized as `installedEngine`.
  final InstalledLocalEngine installedEngine;
  /// Rust field `modelDownloadedBytes` serialized as `modelDownloadedBytes`.
  final int modelDownloadedBytes;
  /// Rust field `engineDownloadedBytes` serialized as `engineDownloadedBytes`.
  final int engineDownloadedBytes;
}

/// Generated Dart model for Rust type `operit_runtime::services::LocalModelService::LocalModelCatalogStatus`.
class LocalModelCatalogStatus {
  const LocalModelCatalogStatus({
    required this.manifest,
    required this.installedModel,
    required this.engineManifest,
    required this.installedEngine,
    required this.platformCompatible,
  });

  factory LocalModelCatalogStatus.fromJson(Map<String, Object?> json) {
    return LocalModelCatalogStatus(
      manifest: LocalModelManifest.fromJson(json['manifest'] as Map<String, Object?>),
      installedModel: json['installedModel'] == null ? null : InstalledLocalModel.fromJson(json['installedModel'] as Map<String, Object?>),
      engineManifest: json['engineManifest'] == null ? null : LocalEngineManifest.fromJson(json['engineManifest'] as Map<String, Object?>),
      installedEngine: json['installedEngine'] == null ? null : InstalledLocalEngine.fromJson(json['installedEngine'] as Map<String, Object?>),
      platformCompatible: json['platformCompatible'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'manifest': manifest.toJson(),
      'installedModel': installedModel?.toJson(),
      'engineManifest': engineManifest?.toJson(),
      'installedEngine': installedEngine?.toJson(),
      'platformCompatible': platformCompatible,
    };
  }

  /// Rust field `manifest` serialized as `manifest`.
  final LocalModelManifest manifest;
  /// Rust field `installedModel` serialized as `installedModel`.
  final InstalledLocalModel? installedModel;
  /// Rust field `engineManifest` serialized as `engineManifest`.
  final LocalEngineManifest? engineManifest;
  /// Rust field `installedEngine` serialized as `installedEngine`.
  final InstalledLocalEngine? installedEngine;
  /// Rust field `platformCompatible` serialized as `platformCompatible`.
  final bool platformCompatible;
}

/// Generated Dart enum for Rust type `operit_runtime::services::LocalModelService::LocalModelInstallPhase`.
enum LocalModelInstallPhase {
  preparing('Preparing'),
  engine('Engine'),
  model('Model'),
  cancelling('Cancelling'),
  cancelled('Cancelled'),
  completed('Completed'),
  failed('Failed'),
  ;

  const LocalModelInstallPhase(this.value);

  final String value;

  factory LocalModelInstallPhase.fromJson(Object? value) {
    return switch (value) {
      'Preparing' => LocalModelInstallPhase.preparing,
      'Engine' => LocalModelInstallPhase.engine,
      'Model' => LocalModelInstallPhase.model,
      'Cancelling' => LocalModelInstallPhase.cancelling,
      'Cancelled' => LocalModelInstallPhase.cancelled,
      'Completed' => LocalModelInstallPhase.completed,
      'Failed' => LocalModelInstallPhase.failed,
      _ => throw ArgumentError('Unknown LocalModelInstallPhase: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_runtime::services::LocalModelService::LocalModelInstallStatus`.
class LocalModelInstallStatus {
  const LocalModelInstallStatus({
    required this.operationId,
    required this.modelId,
    required this.version,
    required this.phase,
    required this.currentFile,
    required this.downloadedBytes,
    required this.totalBytes,
    required this.error,
  });

  factory LocalModelInstallStatus.fromJson(Map<String, Object?> json) {
    return LocalModelInstallStatus(
      operationId: json['operationId'] as String,
      modelId: json['modelId'] as String,
      version: json['version'] as String,
      phase: LocalModelInstallPhase.fromJson(json['phase']),
      currentFile: json['currentFile'] == null ? null : json['currentFile'] as String,
      downloadedBytes: json['downloadedBytes'] as int,
      totalBytes: json['totalBytes'] as int,
      error: json['error'] == null ? null : json['error'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'operationId': operationId,
      'modelId': modelId,
      'version': version,
      'phase': phase.toJson(),
      'currentFile': currentFile,
      'downloadedBytes': downloadedBytes,
      'totalBytes': totalBytes,
      'error': error,
    };
  }

  /// Rust field `operationId` serialized as `operationId`.
  final String operationId;
  /// Rust field `modelId` serialized as `modelId`.
  final String modelId;
  /// Rust field `version` serialized as `version`.
  final String version;
  /// Rust field `phase` serialized as `phase`.
  final LocalModelInstallPhase phase;
  /// Rust field `currentFile` serialized as `currentFile`.
  final String? currentFile;
  /// Rust field `downloadedBytes` serialized as `downloadedBytes`.
  final int downloadedBytes;
  /// Rust field `totalBytes` serialized as `totalBytes`.
  final int totalBytes;
  /// Rust field `error` serialized as `error`.
  final String? error;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeBrowserService::RuntimeBrowserCommand`.
class RuntimeBrowserCommand {
  const RuntimeBrowserCommand({
    required this.action,
    required this.sessionId,
    required this.url,
    required this.script,
    required this.payloadJson,
    required this.userAgent,
    required this.headers,
  });

  factory RuntimeBrowserCommand.fromJson(Map<String, Object?> json) {
    return RuntimeBrowserCommand(
      action: json['action'] as String,
      sessionId: json['sessionId'] == null ? null : json['sessionId'] as String,
      url: json['url'] == null ? null : json['url'] as String,
      script: json['script'] == null ? null : json['script'] as String,
      payloadJson: json['payloadJson'] as String,
      userAgent: json['userAgent'] == null ? null : json['userAgent'] as String,
      headers: (json['headers'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'action': action,
      'sessionId': sessionId,
      'url': url,
      'script': script,
      'payloadJson': payloadJson,
      'userAgent': userAgent,
      'headers': headers.map((key, value) => MapEntry(key, value)),
    };
  }

  /// Rust field `action` serialized as `action`.
  final String action;
  /// Rust field `sessionId` serialized as `sessionId`.
  final String? sessionId;
  /// Rust field `url` serialized as `url`.
  final String? url;
  /// Rust field `script` serialized as `script`.
  final String? script;
  /// Rust field `payloadJson` serialized as `payloadJson`.
  final String payloadJson;
  /// Rust field `userAgent` serialized as `userAgent`.
  final String? userAgent;
  /// Rust field `headers` serialized as `headers`.
  final Map<String, String> headers;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeBrowserService::RuntimeBrowserCommandResult`.
class RuntimeBrowserCommandResult {
  const RuntimeBrowserCommandResult({
    required this.success,
    required this.session,
    required this.sessions,
    required this.resultJson,
    required this.error,
  });

  factory RuntimeBrowserCommandResult.fromJson(Map<String, Object?> json) {
    return RuntimeBrowserCommandResult(
      success: json['success'] as bool,
      session: json['session'] == null ? null : RuntimeBrowserSessionInfo.fromJson(json['session'] as Map<String, Object?>),
      sessions: (json['sessions'] as List<Object?>).map((item) => RuntimeBrowserSessionInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
      resultJson: json['resultJson'] as String,
      error: json['error'] == null ? null : json['error'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'success': success,
      'session': session?.toJson(),
      'sessions': sessions.map((item) => item.toJson()).toList(growable: false),
      'resultJson': resultJson,
      'error': error,
    };
  }

  /// Rust field `success` serialized as `success`.
  final bool success;
  /// Rust field `session` serialized as `session`.
  final RuntimeBrowserSessionInfo? session;
  /// Rust field `sessions` serialized as `sessions`.
  final List<RuntimeBrowserSessionInfo> sessions;
  /// Rust field `resultJson` serialized as `resultJson`.
  final String resultJson;
  /// Rust field `error` serialized as `error`.
  final String? error;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeBrowserService::RuntimeBrowserSessionEvent`.
class RuntimeBrowserSessionEvent {
  const RuntimeBrowserSessionEvent({
    required this.sessionId,
    required this.eventType,
    required this.session,
    required this.resultJson,
    required this.frameData,
    required this.frameCodec,
    required this.frameWidth,
    required this.frameHeight,
    required this.error,
  });

  factory RuntimeBrowserSessionEvent.fromJson(Map<String, Object?> json) {
    return RuntimeBrowserSessionEvent(
      sessionId: json['sessionId'] as String,
      eventType: json['eventType'] as String,
      session: json['session'] == null ? null : RuntimeBrowserSessionInfo.fromJson(json['session'] as Map<String, Object?>),
      resultJson: json['resultJson'] as String,
      frameData: json['frameData'] as Uint8List,
      frameCodec: json['frameCodec'] == null ? null : json['frameCodec'] as String,
      frameWidth: json['frameWidth'] == null ? null : json['frameWidth'] as int,
      frameHeight: json['frameHeight'] == null ? null : json['frameHeight'] as int,
      error: json['error'] == null ? null : json['error'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'eventType': eventType,
      'session': session?.toJson(),
      'resultJson': resultJson,
      'frameData': frameData,
      'frameCodec': frameCodec,
      'frameWidth': frameWidth,
      'frameHeight': frameHeight,
      'error': error,
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `eventType` serialized as `eventType`.
  final String eventType;
  /// Rust field `session` serialized as `session`.
  final RuntimeBrowserSessionInfo? session;
  /// Rust field `resultJson` serialized as `resultJson`.
  final String resultJson;
  /// Rust field `frameData` serialized as `frameData`.
  final Uint8List frameData;
  /// Rust field `frameCodec` serialized as `frameCodec`.
  final String? frameCodec;
  /// Rust field `frameWidth` serialized as `frameWidth`.
  final int? frameWidth;
  /// Rust field `frameHeight` serialized as `frameHeight`.
  final int? frameHeight;
  /// Rust field `error` serialized as `error`.
  final String? error;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeBrowserService::RuntimeBrowserSessionInfo`.
class RuntimeBrowserSessionInfo {
  const RuntimeBrowserSessionInfo({
    required this.sessionId,
    required this.currentUrl,
    required this.title,
    required this.userAgent,
    required this.active,
    required this.canGoBack,
    required this.canGoForward,
    required this.isLoading,
    required this.progress,
  });

  factory RuntimeBrowserSessionInfo.fromJson(Map<String, Object?> json) {
    return RuntimeBrowserSessionInfo(
      sessionId: json['sessionId'] as String,
      currentUrl: json['currentUrl'] as String,
      title: json['title'] as String,
      userAgent: json['userAgent'] == null ? null : json['userAgent'] as String,
      active: json['active'] as bool,
      canGoBack: json['canGoBack'] as bool,
      canGoForward: json['canGoForward'] as bool,
      isLoading: json['isLoading'] as bool,
      progress: json['progress'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'currentUrl': currentUrl,
      'title': title,
      'userAgent': userAgent,
      'active': active,
      'canGoBack': canGoBack,
      'canGoForward': canGoForward,
      'isLoading': isLoading,
      'progress': progress,
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `currentUrl` serialized as `currentUrl`.
  final String currentUrl;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `userAgent` serialized as `userAgent`.
  final String? userAgent;
  /// Rust field `active` serialized as `active`.
  final bool active;
  /// Rust field `canGoBack` serialized as `canGoBack`.
  final bool canGoBack;
  /// Rust field `canGoForward` serialized as `canGoForward`.
  final bool canGoForward;
  /// Rust field `isLoading` serialized as `isLoading`.
  final bool isLoading;
  /// Rust field `progress` serialized as `progress`.
  final int progress;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeBrowserService::RuntimeBrowserSessionSnapshot`.
class RuntimeBrowserSessionSnapshot {
  const RuntimeBrowserSessionSnapshot({
    required this.session,
    required this.resultJson,
  });

  factory RuntimeBrowserSessionSnapshot.fromJson(Map<String, Object?> json) {
    return RuntimeBrowserSessionSnapshot(
      session: RuntimeBrowserSessionInfo.fromJson(json['session'] as Map<String, Object?>),
      resultJson: json['resultJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'session': session.toJson(),
      'resultJson': resultJson,
    };
  }

  /// Rust field `session` serialized as `session`.
  final RuntimeBrowserSessionInfo session;
  /// Rust field `resultJson` serialized as `resultJson`.
  final String resultJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeBrowserService::RuntimeBrowserStreamEvent`.
class RuntimeBrowserStreamEvent {
  const RuntimeBrowserStreamEvent({
    required this.sequence,
    required this.sessionId,
    required this.eventType,
    required this.session,
    required this.resultJson,
    required this.frameData,
    required this.frameCodec,
    required this.frameWidth,
    required this.frameHeight,
    required this.error,
  });

  factory RuntimeBrowserStreamEvent.fromJson(Map<String, Object?> json) {
    return RuntimeBrowserStreamEvent(
      sequence: json['sequence'] as int,
      sessionId: json['sessionId'] as String,
      eventType: json['eventType'] as String,
      session: json['session'] == null ? null : RuntimeBrowserSessionInfo.fromJson(json['session'] as Map<String, Object?>),
      resultJson: json['resultJson'] as String,
      frameData: json['frameData'] as Uint8List,
      frameCodec: json['frameCodec'] == null ? null : json['frameCodec'] as String,
      frameWidth: json['frameWidth'] == null ? null : json['frameWidth'] as int,
      frameHeight: json['frameHeight'] == null ? null : json['frameHeight'] as int,
      error: json['error'] == null ? null : json['error'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sequence': sequence,
      'sessionId': sessionId,
      'eventType': eventType,
      'session': session?.toJson(),
      'resultJson': resultJson,
      'frameData': frameData,
      'frameCodec': frameCodec,
      'frameWidth': frameWidth,
      'frameHeight': frameHeight,
      'error': error,
    };
  }

  /// Rust field `sequence` serialized as `sequence`.
  final int sequence;
  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `eventType` serialized as `eventType`.
  final String eventType;
  /// Rust field `session` serialized as `session`.
  final RuntimeBrowserSessionInfo? session;
  /// Rust field `resultJson` serialized as `resultJson`.
  final String resultJson;
  /// Rust field `frameData` serialized as `frameData`.
  final Uint8List frameData;
  /// Rust field `frameCodec` serialized as `frameCodec`.
  final String? frameCodec;
  /// Rust field `frameWidth` serialized as `frameWidth`.
  final int? frameWidth;
  /// Rust field `frameHeight` serialized as `frameHeight`.
  final int? frameHeight;
  /// Rust field `error` serialized as `error`.
  final String? error;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInfoService::RuntimeHostDescriptor`.
class RuntimeHostDescriptor {
  const RuntimeHostDescriptor({
    required this.id,
    required this.displayName,
    required this.platform,
    required this.privilege,
    required this.isolation,
    required this.pathStyleDescriptionEn,
    required this.pathStyleDescriptionCn,
    required this.examplePaths,
    required this.usesEnvironmentParameter,
    required this.environmentParameterDescriptionEn,
    required this.environmentParameterDescriptionCn,
    required this.capabilities,
    required this.structuredCapabilities,
    required this.onboardingRequirements,
    required this.workspaceRoots,
    required this.fileSystemHost,
    required this.webVisitHost,
    required this.systemOperationHost,
    required this.audioPlaybackHost,
    required this.ttsSynthesisHost,
    required this.ttsPlaybackHost,
    required this.systemTtsPlaybackHost,
    required this.managedRuntimeHost,
    required this.runtimeStorageHost,
    required this.runtimeSqliteHost,
    required this.browserAutomationHost,
    required this.composeDslWebViewHost,
    required this.terminalHost,
    required this.hostRuntimeEventHost,
  });

  factory RuntimeHostDescriptor.fromJson(Map<String, Object?> json) {
    return RuntimeHostDescriptor(
      id: json['id'] as String,
      displayName: json['displayName'] as String,
      platform: HostPlatform.fromJson(json['platform']),
      privilege: HostPrivilege.fromJson(json['privilege']),
      isolation: HostIsolation.fromJson(json['isolation']),
      pathStyleDescriptionEn: json['pathStyleDescriptionEn'] as String,
      pathStyleDescriptionCn: json['pathStyleDescriptionCn'] as String,
      examplePaths: (json['examplePaths'] as List<Object?>).map((item) => item as String).toList(growable: false),
      usesEnvironmentParameter: json['usesEnvironmentParameter'] as bool,
      environmentParameterDescriptionEn: json['environmentParameterDescriptionEn'] as String,
      environmentParameterDescriptionCn: json['environmentParameterDescriptionCn'] as String,
      capabilities: (json['capabilities'] as List<Object?>).map((item) => item as String).toList(growable: false),
      structuredCapabilities: (json['structuredCapabilities'] as List<Object?>).map((item) => HostCapability.fromJson(item as Map<String, Object?>)).toList(growable: false),
      onboardingRequirements: (json['onboardingRequirements'] as List<Object?>).map((item) => HostOnboardingRequirement.fromJson(item as Map<String, Object?>)).toList(growable: false),
      workspaceRoots: (json['workspaceRoots'] as List<Object?>).map((item) => WorkspaceRootDescriptor.fromJson(item as Map<String, Object?>)).toList(growable: false),
      fileSystemHost: json['fileSystemHost'] as bool,
      webVisitHost: json['webVisitHost'] as bool,
      systemOperationHost: json['systemOperationHost'] as bool,
      audioPlaybackHost: json['audioPlaybackHost'] as bool,
      ttsSynthesisHost: json['ttsSynthesisHost'] as bool,
      ttsPlaybackHost: json['ttsPlaybackHost'] as bool,
      systemTtsPlaybackHost: json['systemTtsPlaybackHost'] as bool,
      managedRuntimeHost: json['managedRuntimeHost'] as bool,
      runtimeStorageHost: json['runtimeStorageHost'] as bool,
      runtimeSqliteHost: json['runtimeSqliteHost'] as bool,
      browserAutomationHost: json['browserAutomationHost'] as bool,
      composeDslWebViewHost: json['composeDslWebViewHost'] as bool,
      terminalHost: json['terminalHost'] as bool,
      hostRuntimeEventHost: json['hostRuntimeEventHost'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'displayName': displayName,
      'platform': platform.toJson(),
      'privilege': privilege.toJson(),
      'isolation': isolation.toJson(),
      'pathStyleDescriptionEn': pathStyleDescriptionEn,
      'pathStyleDescriptionCn': pathStyleDescriptionCn,
      'examplePaths': examplePaths.map((item) => item).toList(growable: false),
      'usesEnvironmentParameter': usesEnvironmentParameter,
      'environmentParameterDescriptionEn': environmentParameterDescriptionEn,
      'environmentParameterDescriptionCn': environmentParameterDescriptionCn,
      'capabilities': capabilities.map((item) => item).toList(growable: false),
      'structuredCapabilities': structuredCapabilities.map((item) => item.toJson()).toList(growable: false),
      'onboardingRequirements': onboardingRequirements.map((item) => item.toJson()).toList(growable: false),
      'workspaceRoots': workspaceRoots.map((item) => item.toJson()).toList(growable: false),
      'fileSystemHost': fileSystemHost,
      'webVisitHost': webVisitHost,
      'systemOperationHost': systemOperationHost,
      'audioPlaybackHost': audioPlaybackHost,
      'ttsSynthesisHost': ttsSynthesisHost,
      'ttsPlaybackHost': ttsPlaybackHost,
      'systemTtsPlaybackHost': systemTtsPlaybackHost,
      'managedRuntimeHost': managedRuntimeHost,
      'runtimeStorageHost': runtimeStorageHost,
      'runtimeSqliteHost': runtimeSqliteHost,
      'browserAutomationHost': browserAutomationHost,
      'composeDslWebViewHost': composeDslWebViewHost,
      'terminalHost': terminalHost,
      'hostRuntimeEventHost': hostRuntimeEventHost,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `displayName` serialized as `displayName`.
  final String displayName;
  /// Rust field `platform` serialized as `platform`.
  final HostPlatform platform;
  /// Rust field `privilege` serialized as `privilege`.
  final HostPrivilege privilege;
  /// Rust field `isolation` serialized as `isolation`.
  final HostIsolation isolation;
  /// Rust field `pathStyleDescriptionEn` serialized as `pathStyleDescriptionEn`.
  final String pathStyleDescriptionEn;
  /// Rust field `pathStyleDescriptionCn` serialized as `pathStyleDescriptionCn`.
  final String pathStyleDescriptionCn;
  /// Rust field `examplePaths` serialized as `examplePaths`.
  final List<String> examplePaths;
  /// Rust field `usesEnvironmentParameter` serialized as `usesEnvironmentParameter`.
  final bool usesEnvironmentParameter;
  /// Rust field `environmentParameterDescriptionEn` serialized as `environmentParameterDescriptionEn`.
  final String environmentParameterDescriptionEn;
  /// Rust field `environmentParameterDescriptionCn` serialized as `environmentParameterDescriptionCn`.
  final String environmentParameterDescriptionCn;
  /// Rust field `capabilities` serialized as `capabilities`.
  final List<String> capabilities;
  /// Rust field `structuredCapabilities` serialized as `structuredCapabilities`.
  final List<HostCapability> structuredCapabilities;
  /// Rust field `onboardingRequirements` serialized as `onboardingRequirements`.
  final List<HostOnboardingRequirement> onboardingRequirements;
  /// Rust field `workspaceRoots` serialized as `workspaceRoots`.
  final List<WorkspaceRootDescriptor> workspaceRoots;
  /// Rust field `fileSystemHost` serialized as `fileSystemHost`.
  final bool fileSystemHost;
  /// Rust field `webVisitHost` serialized as `webVisitHost`.
  final bool webVisitHost;
  /// Rust field `systemOperationHost` serialized as `systemOperationHost`.
  final bool systemOperationHost;
  /// Rust field `audioPlaybackHost` serialized as `audioPlaybackHost`.
  final bool audioPlaybackHost;
  /// Rust field `ttsSynthesisHost` serialized as `ttsSynthesisHost`.
  final bool ttsSynthesisHost;
  /// Rust field `ttsPlaybackHost` serialized as `ttsPlaybackHost`.
  final bool ttsPlaybackHost;
  /// Rust field `systemTtsPlaybackHost` serialized as `systemTtsPlaybackHost`.
  final bool systemTtsPlaybackHost;
  /// Rust field `managedRuntimeHost` serialized as `managedRuntimeHost`.
  final bool managedRuntimeHost;
  /// Rust field `runtimeStorageHost` serialized as `runtimeStorageHost`.
  final bool runtimeStorageHost;
  /// Rust field `runtimeSqliteHost` serialized as `runtimeSqliteHost`.
  final bool runtimeSqliteHost;
  /// Rust field `browserAutomationHost` serialized as `browserAutomationHost`.
  final bool browserAutomationHost;
  /// Rust field `composeDslWebViewHost` serialized as `composeDslWebViewHost`.
  final bool composeDslWebViewHost;
  /// Rust field `terminalHost` serialized as `terminalHost`.
  final bool terminalHost;
  /// Rust field `hostRuntimeEventHost` serialized as `hostRuntimeEventHost`.
  final bool hostRuntimeEventHost;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionAppNotificationPayload`.
class RuntimeHostInteractionAppNotificationPayload {
  const RuntimeHostInteractionAppNotificationPayload({
    required this.notificationType,
    required this.title,
    required this.message,
    required this.chatId,
    required this.messageTimestamp,
  });

  factory RuntimeHostInteractionAppNotificationPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionAppNotificationPayload(
      notificationType: json['notificationType'] as String,
      title: json['title'] as String,
      message: json['message'] as String,
      chatId: json['chatId'] == null ? null : json['chatId'] as String,
      messageTimestamp: json['messageTimestamp'] == null ? null : json['messageTimestamp'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'notificationType': notificationType,
      'title': title,
      'message': message,
      'chatId': chatId,
      'messageTimestamp': messageTimestamp,
    };
  }

  /// Rust field `notificationType` serialized as `notificationType`.
  final String notificationType;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `message` serialized as `message`.
  final String message;
  /// Rust field `chatId` serialized as `chatId`.
  final String? chatId;
  /// Rust field `messageTimestamp` serialized as `messageTimestamp`.
  final int? messageTimestamp;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionAudioPlayPayload`.
class RuntimeHostInteractionAudioPlayPayload {
  const RuntimeHostInteractionAudioPlayPayload({
    required this.path,
  });

  factory RuntimeHostInteractionAudioPlayPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionAudioPlayPayload(
      path: json['path'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionAudioPlayResponse`.
class RuntimeHostInteractionAudioPlayResponse {
  const RuntimeHostInteractionAudioPlayResponse({
    required this.path,
    required this.started,
    required this.details,
  });

  factory RuntimeHostInteractionAudioPlayResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionAudioPlayResponse(
      path: json['path'] as String,
      started: json['started'] as bool,
      details: json['details'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'started': started,
      'details': details,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `started` serialized as `started`.
  final bool started;
  /// Rust field `details` serialized as `details`.
  final String details;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionBluetoothPayload`.
class RuntimeHostInteractionBluetoothPayload {
  const RuntimeHostInteractionBluetoothPayload({
    required this.command,
    required this.paramsJson,
  });

  factory RuntimeHostInteractionBluetoothPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionBluetoothPayload(
      command: json['command'] as String,
      paramsJson: json['paramsJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'command': command,
      'paramsJson': paramsJson,
    };
  }

  /// Rust field `command` serialized as `command`.
  final String command;
  /// Rust field `paramsJson` serialized as `paramsJson`.
  final String paramsJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionBluetoothResponse`.
class RuntimeHostInteractionBluetoothResponse {
  const RuntimeHostInteractionBluetoothResponse({
    required this.resultJson,
  });

  factory RuntimeHostInteractionBluetoothResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionBluetoothResponse(
      resultJson: json['resultJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'resultJson': resultJson,
    };
  }

  /// Rust field `resultJson` serialized as `resultJson`.
  final String resultJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionBrowserAutomationPayload`.
class RuntimeHostInteractionBrowserAutomationPayload {
  const RuntimeHostInteractionBrowserAutomationPayload({
    required this.requestId,
    required this.toolName,
    required this.parametersJson,
    required this.requestedAtMillis,
  });

  factory RuntimeHostInteractionBrowserAutomationPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionBrowserAutomationPayload(
      requestId: json['requestId'] as String,
      toolName: json['toolName'] as String,
      parametersJson: json['parametersJson'] as String,
      requestedAtMillis: json['requestedAtMillis'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'requestId': requestId,
      'toolName': toolName,
      'parametersJson': parametersJson,
      'requestedAtMillis': requestedAtMillis,
    };
  }

  /// Rust field `requestId` serialized as `requestId`.
  final String requestId;
  /// Rust field `toolName` serialized as `toolName`.
  final String toolName;
  /// Rust field `parametersJson` serialized as `parametersJson`.
  final String parametersJson;
  /// Rust field `requestedAtMillis` serialized as `requestedAtMillis`.
  final int requestedAtMillis;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionBrowserAutomationResponse`.
class RuntimeHostInteractionBrowserAutomationResponse {
  const RuntimeHostInteractionBrowserAutomationResponse({
    required this.requestId,
    required this.success,
    required this.result,
    required this.error,
  });

  factory RuntimeHostInteractionBrowserAutomationResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionBrowserAutomationResponse(
      requestId: json['requestId'] as String,
      success: json['success'] as bool,
      result: json['result'] as String,
      error: json['error'] == null ? null : json['error'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'requestId': requestId,
      'success': success,
      'result': result,
      'error': error,
    };
  }

  /// Rust field `requestId` serialized as `requestId`.
  final String requestId;
  /// Rust field `success` serialized as `success`.
  final bool success;
  /// Rust field `result` serialized as `result`.
  final String result;
  /// Rust field `error` serialized as `error`.
  final String? error;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionBrowserSessionPayload`.
class RuntimeHostInteractionBrowserSessionPayload {
  const RuntimeHostInteractionBrowserSessionPayload({
    required this.commandJson,
  });

  factory RuntimeHostInteractionBrowserSessionPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionBrowserSessionPayload(
      commandJson: json['commandJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'commandJson': commandJson,
    };
  }

  /// Rust field `commandJson` serialized as `commandJson`.
  final String commandJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionBrowserSessionResponse`.
class RuntimeHostInteractionBrowserSessionResponse {
  const RuntimeHostInteractionBrowserSessionResponse({
    required this.resultJson,
  });

  factory RuntimeHostInteractionBrowserSessionResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionBrowserSessionResponse(
      resultJson: json['resultJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'resultJson': resultJson,
    };
  }

  /// Rust field `resultJson` serialized as `resultJson`.
  final String resultJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionComposeFilePickerPayload`.
class RuntimeHostInteractionComposeFilePickerPayload {
  const RuntimeHostInteractionComposeFilePickerPayload({
    required this.requestJson,
  });

  factory RuntimeHostInteractionComposeFilePickerPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionComposeFilePickerPayload(
      requestJson: json['requestJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'requestJson': requestJson,
    };
  }

  /// Rust field `requestJson` serialized as `requestJson`.
  final String requestJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionComposeFilePickerResponse`.
class RuntimeHostInteractionComposeFilePickerResponse {
  const RuntimeHostInteractionComposeFilePickerResponse({
    required this.resultJson,
  });

  factory RuntimeHostInteractionComposeFilePickerResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionComposeFilePickerResponse(
      resultJson: json['resultJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'resultJson': resultJson,
    };
  }

  /// Rust field `resultJson` serialized as `resultJson`.
  final String resultJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionComposeWebViewControllerPayload`.
class RuntimeHostInteractionComposeWebViewControllerPayload {
  const RuntimeHostInteractionComposeWebViewControllerPayload({
    required this.commandJson,
  });

  factory RuntimeHostInteractionComposeWebViewControllerPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionComposeWebViewControllerPayload(
      commandJson: json['commandJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'commandJson': commandJson,
    };
  }

  /// Rust field `commandJson` serialized as `commandJson`.
  final String commandJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionComposeWebViewControllerResponse`.
class RuntimeHostInteractionComposeWebViewControllerResponse {
  const RuntimeHostInteractionComposeWebViewControllerResponse({
    required this.result,
  });

  factory RuntimeHostInteractionComposeWebViewControllerResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionComposeWebViewControllerResponse(
      result: json['result'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'result': result,
    };
  }

  /// Rust field `result` serialized as `result`.
  final String result;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionFileOpenPayload`.
class RuntimeHostInteractionFileOpenPayload {
  const RuntimeHostInteractionFileOpenPayload({
    required this.path,
  });

  factory RuntimeHostInteractionFileOpenPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionFileOpenPayload(
      path: json['path'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionFileOperationResponse`.
class RuntimeHostInteractionFileOperationResponse {
  const RuntimeHostInteractionFileOperationResponse({
    required this.success,
    required this.error,
  });

  factory RuntimeHostInteractionFileOperationResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionFileOperationResponse(
      success: json['success'] as bool,
      error: json['error'] == null ? null : json['error'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'success': success,
      'error': error,
    };
  }

  /// Rust field `success` serialized as `success`.
  final bool success;
  /// Rust field `error` serialized as `error`.
  final String? error;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionFileSharePayload`.
class RuntimeHostInteractionFileSharePayload {
  const RuntimeHostInteractionFileSharePayload({
    required this.path,
    required this.title,
  });

  factory RuntimeHostInteractionFileSharePayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionFileSharePayload(
      path: json['path'] as String,
      title: json['title'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'title': title,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `title` serialized as `title`.
  final String title;
}

/// Generated Dart enum for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionKind`.
enum RuntimeHostInteractionKind {
  browserAutomation('browser_automation'),
  browserSession('browser_session'),
  webVisit('web_visit'),
  composeWebViewController('compose_webview_controller'),
  composeFilePicker('compose_file_picker'),
  systemCaptureScreenshot('system_capture_screenshot'),
  systemLanguageCode('system_language_code'),
  systemRecognizeText('system_recognize_text'),
  systemOperation('system_operation'),
  fileOpen('file_open'),
  fileShare('file_share'),
  audioPlay('audio_play'),
  musicPlayback('music_playback'),
  bluetooth('bluetooth'),
  ttsSynthesis('tts_synthesis'),
  ttsPlayback('tts_playback'),
  localInference('local_inference'),
  toolPermission('tool_permission'),
  webAccessPairing('web_access_pairing'),
  appNotification('app_notification'),
  ;

  const RuntimeHostInteractionKind(this.value);

  final String value;

  factory RuntimeHostInteractionKind.fromJson(Object? value) {
    return switch (value) {
      'browser_automation' => RuntimeHostInteractionKind.browserAutomation,
      'browser_session' => RuntimeHostInteractionKind.browserSession,
      'web_visit' => RuntimeHostInteractionKind.webVisit,
      'compose_webview_controller' => RuntimeHostInteractionKind.composeWebViewController,
      'compose_file_picker' => RuntimeHostInteractionKind.composeFilePicker,
      'system_capture_screenshot' => RuntimeHostInteractionKind.systemCaptureScreenshot,
      'system_language_code' => RuntimeHostInteractionKind.systemLanguageCode,
      'system_recognize_text' => RuntimeHostInteractionKind.systemRecognizeText,
      'system_operation' => RuntimeHostInteractionKind.systemOperation,
      'file_open' => RuntimeHostInteractionKind.fileOpen,
      'file_share' => RuntimeHostInteractionKind.fileShare,
      'audio_play' => RuntimeHostInteractionKind.audioPlay,
      'music_playback' => RuntimeHostInteractionKind.musicPlayback,
      'bluetooth' => RuntimeHostInteractionKind.bluetooth,
      'tts_synthesis' => RuntimeHostInteractionKind.ttsSynthesis,
      'tts_playback' => RuntimeHostInteractionKind.ttsPlayback,
      'local_inference' => RuntimeHostInteractionKind.localInference,
      'tool_permission' => RuntimeHostInteractionKind.toolPermission,
      'web_access_pairing' => RuntimeHostInteractionKind.webAccessPairing,
      'app_notification' => RuntimeHostInteractionKind.appNotification,
      _ => throw ArgumentError('Unknown RuntimeHostInteractionKind: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionLocalInferencePayload`.
class RuntimeHostInteractionLocalInferencePayload {
  const RuntimeHostInteractionLocalInferencePayload({
    required this.method,
    required this.requestJson,
  });

  factory RuntimeHostInteractionLocalInferencePayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionLocalInferencePayload(
      method: json['method'] as String,
      requestJson: json['requestJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'method': method,
      'requestJson': requestJson,
    };
  }

  /// Rust field `method` serialized as `method`.
  final String method;
  /// Rust field `requestJson` serialized as `requestJson`.
  final String requestJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionLocalInferenceResponse`.
class RuntimeHostInteractionLocalInferenceResponse {
  const RuntimeHostInteractionLocalInferenceResponse({
    required this.resultJson,
  });

  factory RuntimeHostInteractionLocalInferenceResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionLocalInferenceResponse(
      resultJson: json['resultJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'resultJson': resultJson,
    };
  }

  /// Rust field `resultJson` serialized as `resultJson`.
  final String resultJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionMusicPlaybackPayload`.
class RuntimeHostInteractionMusicPlaybackPayload {
  const RuntimeHostInteractionMusicPlaybackPayload({
    required this.command,
    required this.source,
    required this.sourceType,
    required this.title,
    required this.artist,
    required this.loopPlayback,
    required this.volume,
    required this.positionMs,
  });

  factory RuntimeHostInteractionMusicPlaybackPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionMusicPlaybackPayload(
      command: json['command'] as String,
      source: json['source'] == null ? null : json['source'] as String,
      sourceType: json['sourceType'] == null ? null : json['sourceType'] as String,
      title: json['title'] == null ? null : json['title'] as String,
      artist: json['artist'] == null ? null : json['artist'] as String,
      loopPlayback: json['loopPlayback'] as bool,
      volume: (json['volume'] as num).toDouble(),
      positionMs: json['positionMs'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'command': command,
      'source': source,
      'sourceType': sourceType,
      'title': title,
      'artist': artist,
      'loopPlayback': loopPlayback,
      'volume': volume,
      'positionMs': positionMs,
    };
  }

  /// Rust field `command` serialized as `command`.
  final String command;
  /// Rust field `source` serialized as `source`.
  final String? source;
  /// Rust field `sourceType` serialized as `sourceType`.
  final String? sourceType;
  /// Rust field `title` serialized as `title`.
  final String? title;
  /// Rust field `artist` serialized as `artist`.
  final String? artist;
  /// Rust field `loopPlayback` serialized as `loopPlayback`.
  final bool loopPlayback;
  /// Rust field `volume` serialized as `volume`.
  final double volume;
  /// Rust field `positionMs` serialized as `positionMs`.
  final int positionMs;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionMusicPlaybackResponse`.
class RuntimeHostInteractionMusicPlaybackResponse {
  const RuntimeHostInteractionMusicPlaybackResponse({
    required this.state,
    required this.source,
    required this.sourceType,
    required this.title,
    required this.artist,
    required this.durationMs,
    required this.positionMs,
    required this.bufferedPositionMs,
    required this.volume,
    required this.loopPlayback,
    required this.message,
  });

  factory RuntimeHostInteractionMusicPlaybackResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionMusicPlaybackResponse(
      state: json['state'] as String,
      source: json['source'] == null ? null : json['source'] as String,
      sourceType: json['sourceType'] == null ? null : json['sourceType'] as String,
      title: json['title'] == null ? null : json['title'] as String,
      artist: json['artist'] == null ? null : json['artist'] as String,
      durationMs: json['durationMs'] == null ? null : json['durationMs'] as int,
      positionMs: json['positionMs'] as int,
      bufferedPositionMs: json['bufferedPositionMs'] as int,
      volume: (json['volume'] as num).toDouble(),
      loopPlayback: json['loopPlayback'] as bool,
      message: json['message'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'state': state,
      'source': source,
      'sourceType': sourceType,
      'title': title,
      'artist': artist,
      'durationMs': durationMs,
      'positionMs': positionMs,
      'bufferedPositionMs': bufferedPositionMs,
      'volume': volume,
      'loopPlayback': loopPlayback,
      'message': message,
    };
  }

  /// Rust field `state` serialized as `state`.
  final String state;
  /// Rust field `source` serialized as `source`.
  final String? source;
  /// Rust field `sourceType` serialized as `sourceType`.
  final String? sourceType;
  /// Rust field `title` serialized as `title`.
  final String? title;
  /// Rust field `artist` serialized as `artist`.
  final String? artist;
  /// Rust field `durationMs` serialized as `durationMs`.
  final int? durationMs;
  /// Rust field `positionMs` serialized as `positionMs`.
  final int positionMs;
  /// Rust field `bufferedPositionMs` serialized as `bufferedPositionMs`.
  final int bufferedPositionMs;
  /// Rust field `volume` serialized as `volume`.
  final double volume;
  /// Rust field `loopPlayback` serialized as `loopPlayback`.
  final bool loopPlayback;
  /// Rust field `message` serialized as `message`.
  final String message;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionRequest`.
class RuntimeHostInteractionRequest {
  const RuntimeHostInteractionRequest({
    required this.requestId,
    required this.kind,
    required this.browserAutomation,
    required this.browserSession,
    required this.webVisit,
    required this.composeWebViewController,
    required this.composeFilePicker,
    required this.systemCaptureScreenshot,
    required this.systemLanguageCode,
    required this.systemRecognizeText,
    required this.systemOperation,
    required this.fileOpen,
    required this.fileShare,
    required this.audioPlay,
    required this.musicPlayback,
    required this.bluetooth,
    required this.ttsSynthesis,
    required this.ttsPlayback,
    required this.localInference,
    required this.toolPermission,
    required this.webAccessPairing,
    required this.appNotification,
  });

  factory RuntimeHostInteractionRequest.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionRequest(
      requestId: json['requestId'] as String,
      kind: RuntimeHostInteractionKind.fromJson(json['kind']),
      browserAutomation: json['browserAutomation'] == null ? null : RuntimeHostInteractionBrowserAutomationPayload.fromJson(json['browserAutomation'] as Map<String, Object?>),
      browserSession: json['browserSession'] == null ? null : RuntimeHostInteractionBrowserSessionPayload.fromJson(json['browserSession'] as Map<String, Object?>),
      webVisit: json['webVisit'] == null ? null : RuntimeHostInteractionWebVisitPayload.fromJson(json['webVisit'] as Map<String, Object?>),
      composeWebViewController: json['composeWebViewController'] == null ? null : RuntimeHostInteractionComposeWebViewControllerPayload.fromJson(json['composeWebViewController'] as Map<String, Object?>),
      composeFilePicker: json['composeFilePicker'] == null ? null : RuntimeHostInteractionComposeFilePickerPayload.fromJson(json['composeFilePicker'] as Map<String, Object?>),
      systemCaptureScreenshot: json['systemCaptureScreenshot'] == null ? null : RuntimeHostInteractionSystemCaptureScreenshotPayload.fromJson(json['systemCaptureScreenshot'] as Map<String, Object?>),
      systemLanguageCode: json['systemLanguageCode'] == null ? null : RuntimeHostInteractionSystemLanguageCodePayload.fromJson(json['systemLanguageCode'] as Map<String, Object?>),
      systemRecognizeText: json['systemRecognizeText'] == null ? null : RuntimeHostInteractionSystemRecognizeTextPayload.fromJson(json['systemRecognizeText'] as Map<String, Object?>),
      systemOperation: json['systemOperation'] == null ? null : RuntimeHostInteractionSystemOperationPayload.fromJson(json['systemOperation'] as Map<String, Object?>),
      fileOpen: json['fileOpen'] == null ? null : RuntimeHostInteractionFileOpenPayload.fromJson(json['fileOpen'] as Map<String, Object?>),
      fileShare: json['fileShare'] == null ? null : RuntimeHostInteractionFileSharePayload.fromJson(json['fileShare'] as Map<String, Object?>),
      audioPlay: json['audioPlay'] == null ? null : RuntimeHostInteractionAudioPlayPayload.fromJson(json['audioPlay'] as Map<String, Object?>),
      musicPlayback: json['musicPlayback'] == null ? null : RuntimeHostInteractionMusicPlaybackPayload.fromJson(json['musicPlayback'] as Map<String, Object?>),
      bluetooth: json['bluetooth'] == null ? null : RuntimeHostInteractionBluetoothPayload.fromJson(json['bluetooth'] as Map<String, Object?>),
      ttsSynthesis: json['ttsSynthesis'] == null ? null : RuntimeHostInteractionTtsSynthesisPayload.fromJson(json['ttsSynthesis'] as Map<String, Object?>),
      ttsPlayback: json['ttsPlayback'] == null ? null : RuntimeHostInteractionTtsPlaybackPayload.fromJson(json['ttsPlayback'] as Map<String, Object?>),
      localInference: json['localInference'] == null ? null : RuntimeHostInteractionLocalInferencePayload.fromJson(json['localInference'] as Map<String, Object?>),
      toolPermission: json['toolPermission'] == null ? null : RuntimeHostInteractionToolPermissionPayload.fromJson(json['toolPermission'] as Map<String, Object?>),
      webAccessPairing: json['webAccessPairing'] == null ? null : RuntimeHostInteractionWebAccessPairingPayload.fromJson(json['webAccessPairing'] as Map<String, Object?>),
      appNotification: json['appNotification'] == null ? null : RuntimeHostInteractionAppNotificationPayload.fromJson(json['appNotification'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'requestId': requestId,
      'kind': kind.toJson(),
      'browserAutomation': browserAutomation?.toJson(),
      'browserSession': browserSession?.toJson(),
      'webVisit': webVisit?.toJson(),
      'composeWebViewController': composeWebViewController?.toJson(),
      'composeFilePicker': composeFilePicker?.toJson(),
      'systemCaptureScreenshot': systemCaptureScreenshot?.toJson(),
      'systemLanguageCode': systemLanguageCode?.toJson(),
      'systemRecognizeText': systemRecognizeText?.toJson(),
      'systemOperation': systemOperation?.toJson(),
      'fileOpen': fileOpen?.toJson(),
      'fileShare': fileShare?.toJson(),
      'audioPlay': audioPlay?.toJson(),
      'musicPlayback': musicPlayback?.toJson(),
      'bluetooth': bluetooth?.toJson(),
      'ttsSynthesis': ttsSynthesis?.toJson(),
      'ttsPlayback': ttsPlayback?.toJson(),
      'localInference': localInference?.toJson(),
      'toolPermission': toolPermission?.toJson(),
      'webAccessPairing': webAccessPairing?.toJson(),
      'appNotification': appNotification?.toJson(),
    };
  }

  /// Rust field `requestId` serialized as `requestId`.
  final String requestId;
  /// Rust field `kind` serialized as `kind`.
  final RuntimeHostInteractionKind kind;
  /// Rust field `browserAutomation` serialized as `browserAutomation`.
  final RuntimeHostInteractionBrowserAutomationPayload? browserAutomation;
  /// Rust field `browserSession` serialized as `browserSession`.
  final RuntimeHostInteractionBrowserSessionPayload? browserSession;
  /// Rust field `webVisit` serialized as `webVisit`.
  final RuntimeHostInteractionWebVisitPayload? webVisit;
  /// Rust field `composeWebViewController` serialized as `composeWebViewController`.
  final RuntimeHostInteractionComposeWebViewControllerPayload? composeWebViewController;
  /// Rust field `composeFilePicker` serialized as `composeFilePicker`.
  final RuntimeHostInteractionComposeFilePickerPayload? composeFilePicker;
  /// Rust field `systemCaptureScreenshot` serialized as `systemCaptureScreenshot`.
  final RuntimeHostInteractionSystemCaptureScreenshotPayload? systemCaptureScreenshot;
  /// Rust field `systemLanguageCode` serialized as `systemLanguageCode`.
  final RuntimeHostInteractionSystemLanguageCodePayload? systemLanguageCode;
  /// Rust field `systemRecognizeText` serialized as `systemRecognizeText`.
  final RuntimeHostInteractionSystemRecognizeTextPayload? systemRecognizeText;
  /// Rust field `systemOperation` serialized as `systemOperation`.
  final RuntimeHostInteractionSystemOperationPayload? systemOperation;
  /// Rust field `fileOpen` serialized as `fileOpen`.
  final RuntimeHostInteractionFileOpenPayload? fileOpen;
  /// Rust field `fileShare` serialized as `fileShare`.
  final RuntimeHostInteractionFileSharePayload? fileShare;
  /// Rust field `audioPlay` serialized as `audioPlay`.
  final RuntimeHostInteractionAudioPlayPayload? audioPlay;
  /// Rust field `musicPlayback` serialized as `musicPlayback`.
  final RuntimeHostInteractionMusicPlaybackPayload? musicPlayback;
  /// Rust field `bluetooth` serialized as `bluetooth`.
  final RuntimeHostInteractionBluetoothPayload? bluetooth;
  /// Rust field `ttsSynthesis` serialized as `ttsSynthesis`.
  final RuntimeHostInteractionTtsSynthesisPayload? ttsSynthesis;
  /// Rust field `ttsPlayback` serialized as `ttsPlayback`.
  final RuntimeHostInteractionTtsPlaybackPayload? ttsPlayback;
  /// Rust field `localInference` serialized as `localInference`.
  final RuntimeHostInteractionLocalInferencePayload? localInference;
  /// Rust field `toolPermission` serialized as `toolPermission`.
  final RuntimeHostInteractionToolPermissionPayload? toolPermission;
  /// Rust field `webAccessPairing` serialized as `webAccessPairing`.
  final RuntimeHostInteractionWebAccessPairingPayload? webAccessPairing;
  /// Rust field `appNotification` serialized as `appNotification`.
  final RuntimeHostInteractionAppNotificationPayload? appNotification;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionResponse`.
class RuntimeHostInteractionResponse {
  const RuntimeHostInteractionResponse({
    required this.error,
    required this.browserAutomation,
    required this.browserSession,
    required this.webVisit,
    required this.composeWebViewController,
    required this.composeFilePicker,
    required this.systemCaptureScreenshot,
    required this.systemLanguageCode,
    required this.systemRecognizeText,
    required this.systemOperation,
    required this.fileOpen,
    required this.fileShare,
    required this.audioPlay,
    required this.musicPlayback,
    required this.bluetooth,
    required this.ttsSynthesis,
    required this.ttsPlayback,
    required this.localInference,
    required this.toolPermission,
  });

  factory RuntimeHostInteractionResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionResponse(
      error: json['error'] == null ? null : json['error'] as String,
      browserAutomation: json['browserAutomation'] == null ? null : RuntimeHostInteractionBrowserAutomationResponse.fromJson(json['browserAutomation'] as Map<String, Object?>),
      browserSession: json['browserSession'] == null ? null : RuntimeHostInteractionBrowserSessionResponse.fromJson(json['browserSession'] as Map<String, Object?>),
      webVisit: json['webVisit'] == null ? null : RuntimeHostInteractionWebVisitResponse.fromJson(json['webVisit'] as Map<String, Object?>),
      composeWebViewController: json['composeWebViewController'] == null ? null : RuntimeHostInteractionComposeWebViewControllerResponse.fromJson(json['composeWebViewController'] as Map<String, Object?>),
      composeFilePicker: json['composeFilePicker'] == null ? null : RuntimeHostInteractionComposeFilePickerResponse.fromJson(json['composeFilePicker'] as Map<String, Object?>),
      systemCaptureScreenshot: json['systemCaptureScreenshot'] == null ? null : RuntimeHostInteractionSystemCaptureScreenshotResponse.fromJson(json['systemCaptureScreenshot'] as Map<String, Object?>),
      systemLanguageCode: json['systemLanguageCode'] == null ? null : RuntimeHostInteractionSystemLanguageCodeResponse.fromJson(json['systemLanguageCode'] as Map<String, Object?>),
      systemRecognizeText: json['systemRecognizeText'] == null ? null : RuntimeHostInteractionSystemRecognizeTextResponse.fromJson(json['systemRecognizeText'] as Map<String, Object?>),
      systemOperation: json['systemOperation'] == null ? null : RuntimeHostInteractionSystemOperationResponse.fromJson(json['systemOperation'] as Map<String, Object?>),
      fileOpen: json['fileOpen'] == null ? null : RuntimeHostInteractionFileOperationResponse.fromJson(json['fileOpen'] as Map<String, Object?>),
      fileShare: json['fileShare'] == null ? null : RuntimeHostInteractionFileOperationResponse.fromJson(json['fileShare'] as Map<String, Object?>),
      audioPlay: json['audioPlay'] == null ? null : RuntimeHostInteractionAudioPlayResponse.fromJson(json['audioPlay'] as Map<String, Object?>),
      musicPlayback: json['musicPlayback'] == null ? null : RuntimeHostInteractionMusicPlaybackResponse.fromJson(json['musicPlayback'] as Map<String, Object?>),
      bluetooth: json['bluetooth'] == null ? null : RuntimeHostInteractionBluetoothResponse.fromJson(json['bluetooth'] as Map<String, Object?>),
      ttsSynthesis: json['ttsSynthesis'] == null ? null : RuntimeHostInteractionTtsSynthesisResponse.fromJson(json['ttsSynthesis'] as Map<String, Object?>),
      ttsPlayback: json['ttsPlayback'] == null ? null : RuntimeHostInteractionTtsPlaybackResponse.fromJson(json['ttsPlayback'] as Map<String, Object?>),
      localInference: json['localInference'] == null ? null : RuntimeHostInteractionLocalInferenceResponse.fromJson(json['localInference'] as Map<String, Object?>),
      toolPermission: json['toolPermission'] == null ? null : RuntimeHostInteractionToolPermissionResponse.fromJson(json['toolPermission'] as Map<String, Object?>),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'error': error,
      'browserAutomation': browserAutomation?.toJson(),
      'browserSession': browserSession?.toJson(),
      'webVisit': webVisit?.toJson(),
      'composeWebViewController': composeWebViewController?.toJson(),
      'composeFilePicker': composeFilePicker?.toJson(),
      'systemCaptureScreenshot': systemCaptureScreenshot?.toJson(),
      'systemLanguageCode': systemLanguageCode?.toJson(),
      'systemRecognizeText': systemRecognizeText?.toJson(),
      'systemOperation': systemOperation?.toJson(),
      'fileOpen': fileOpen?.toJson(),
      'fileShare': fileShare?.toJson(),
      'audioPlay': audioPlay?.toJson(),
      'musicPlayback': musicPlayback?.toJson(),
      'bluetooth': bluetooth?.toJson(),
      'ttsSynthesis': ttsSynthesis?.toJson(),
      'ttsPlayback': ttsPlayback?.toJson(),
      'localInference': localInference?.toJson(),
      'toolPermission': toolPermission?.toJson(),
    };
  }

  /// Rust field `error` serialized as `error`.
  final String? error;
  /// Rust field `browserAutomation` serialized as `browserAutomation`.
  final RuntimeHostInteractionBrowserAutomationResponse? browserAutomation;
  /// Rust field `browserSession` serialized as `browserSession`.
  final RuntimeHostInteractionBrowserSessionResponse? browserSession;
  /// Rust field `webVisit` serialized as `webVisit`.
  final RuntimeHostInteractionWebVisitResponse? webVisit;
  /// Rust field `composeWebViewController` serialized as `composeWebViewController`.
  final RuntimeHostInteractionComposeWebViewControllerResponse? composeWebViewController;
  /// Rust field `composeFilePicker` serialized as `composeFilePicker`.
  final RuntimeHostInteractionComposeFilePickerResponse? composeFilePicker;
  /// Rust field `systemCaptureScreenshot` serialized as `systemCaptureScreenshot`.
  final RuntimeHostInteractionSystemCaptureScreenshotResponse? systemCaptureScreenshot;
  /// Rust field `systemLanguageCode` serialized as `systemLanguageCode`.
  final RuntimeHostInteractionSystemLanguageCodeResponse? systemLanguageCode;
  /// Rust field `systemRecognizeText` serialized as `systemRecognizeText`.
  final RuntimeHostInteractionSystemRecognizeTextResponse? systemRecognizeText;
  /// Rust field `systemOperation` serialized as `systemOperation`.
  final RuntimeHostInteractionSystemOperationResponse? systemOperation;
  /// Rust field `fileOpen` serialized as `fileOpen`.
  final RuntimeHostInteractionFileOperationResponse? fileOpen;
  /// Rust field `fileShare` serialized as `fileShare`.
  final RuntimeHostInteractionFileOperationResponse? fileShare;
  /// Rust field `audioPlay` serialized as `audioPlay`.
  final RuntimeHostInteractionAudioPlayResponse? audioPlay;
  /// Rust field `musicPlayback` serialized as `musicPlayback`.
  final RuntimeHostInteractionMusicPlaybackResponse? musicPlayback;
  /// Rust field `bluetooth` serialized as `bluetooth`.
  final RuntimeHostInteractionBluetoothResponse? bluetooth;
  /// Rust field `ttsSynthesis` serialized as `ttsSynthesis`.
  final RuntimeHostInteractionTtsSynthesisResponse? ttsSynthesis;
  /// Rust field `ttsPlayback` serialized as `ttsPlayback`.
  final RuntimeHostInteractionTtsPlaybackResponse? ttsPlayback;
  /// Rust field `localInference` serialized as `localInference`.
  final RuntimeHostInteractionLocalInferenceResponse? localInference;
  /// Rust field `toolPermission` serialized as `toolPermission`.
  final RuntimeHostInteractionToolPermissionResponse? toolPermission;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionSystemCaptureScreenshotPayload`.
class RuntimeHostInteractionSystemCaptureScreenshotPayload {
  const RuntimeHostInteractionSystemCaptureScreenshotPayload();

  factory RuntimeHostInteractionSystemCaptureScreenshotPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionSystemCaptureScreenshotPayload();
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
    };
  }

}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionSystemCaptureScreenshotResponse`.
class RuntimeHostInteractionSystemCaptureScreenshotResponse {
  const RuntimeHostInteractionSystemCaptureScreenshotResponse({
    required this.path,
  });

  factory RuntimeHostInteractionSystemCaptureScreenshotResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionSystemCaptureScreenshotResponse(
      path: json['path'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionSystemLanguageCodePayload`.
class RuntimeHostInteractionSystemLanguageCodePayload {
  const RuntimeHostInteractionSystemLanguageCodePayload();

  factory RuntimeHostInteractionSystemLanguageCodePayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionSystemLanguageCodePayload();
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
    };
  }

}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionSystemLanguageCodeResponse`.
class RuntimeHostInteractionSystemLanguageCodeResponse {
  const RuntimeHostInteractionSystemLanguageCodeResponse({
    required this.languageCode,
  });

  factory RuntimeHostInteractionSystemLanguageCodeResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionSystemLanguageCodeResponse(
      languageCode: json['languageCode'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'languageCode': languageCode,
    };
  }

  /// Rust field `languageCode` serialized as `languageCode`.
  final String languageCode;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionSystemOperationPayload`.
class RuntimeHostInteractionSystemOperationPayload {
  const RuntimeHostInteractionSystemOperationPayload({
    required this.operation,
    required this.paramsJson,
  });

  factory RuntimeHostInteractionSystemOperationPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionSystemOperationPayload(
      operation: json['operation'] as String,
      paramsJson: json['paramsJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'operation': operation,
      'paramsJson': paramsJson,
    };
  }

  /// Rust field `operation` serialized as `operation`.
  final String operation;
  /// Rust field `paramsJson` serialized as `paramsJson`.
  final String paramsJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionSystemOperationResponse`.
class RuntimeHostInteractionSystemOperationResponse {
  const RuntimeHostInteractionSystemOperationResponse({
    required this.resultJson,
  });

  factory RuntimeHostInteractionSystemOperationResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionSystemOperationResponse(
      resultJson: json['resultJson'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'resultJson': resultJson,
    };
  }

  /// Rust field `resultJson` serialized as `resultJson`.
  final String resultJson;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionSystemRecognizeTextPayload`.
class RuntimeHostInteractionSystemRecognizeTextPayload {
  const RuntimeHostInteractionSystemRecognizeTextPayload({
    required this.imagePath,
    required this.language,
    required this.quality,
  });

  factory RuntimeHostInteractionSystemRecognizeTextPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionSystemRecognizeTextPayload(
      imagePath: json['imagePath'] as String,
      language: json['language'] as String,
      quality: json['quality'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'imagePath': imagePath,
      'language': language,
      'quality': quality,
    };
  }

  /// Rust field `imagePath` serialized as `imagePath`.
  final String imagePath;
  /// Rust field `language` serialized as `language`.
  final String language;
  /// Rust field `quality` serialized as `quality`.
  final String quality;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionSystemRecognizeTextResponse`.
class RuntimeHostInteractionSystemRecognizeTextResponse {
  const RuntimeHostInteractionSystemRecognizeTextResponse({
    required this.text,
  });

  factory RuntimeHostInteractionSystemRecognizeTextResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionSystemRecognizeTextResponse(
      text: json['text'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'text': text,
    };
  }

  /// Rust field `text` serialized as `text`.
  final String text;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionToolPermissionPayload`.
class RuntimeHostInteractionToolPermissionPayload {
  const RuntimeHostInteractionToolPermissionPayload({
    required this.tool,
    required this.description,
  });

  factory RuntimeHostInteractionToolPermissionPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionToolPermissionPayload(
      tool: RuntimeHostInteractionToolPermissionTool.fromJson(json['tool'] as Map<String, Object?>),
      description: json['description'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'tool': tool.toJson(),
      'description': description,
    };
  }

  /// Rust field `tool` serialized as `tool`.
  final RuntimeHostInteractionToolPermissionTool tool;
  /// Rust field `description` serialized as `description`.
  final String description;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionToolPermissionResponse`.
class RuntimeHostInteractionToolPermissionResponse {
  const RuntimeHostInteractionToolPermissionResponse({
    required this.result,
  });

  factory RuntimeHostInteractionToolPermissionResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionToolPermissionResponse(
      result: json['result'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'result': result,
    };
  }

  /// Rust field `result` serialized as `result`.
  final String result;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionToolPermissionTool`.
class RuntimeHostInteractionToolPermissionTool {
  const RuntimeHostInteractionToolPermissionTool({
    required this.name,
    required this.parameters,
  });

  factory RuntimeHostInteractionToolPermissionTool.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionToolPermissionTool(
      name: json['name'] as String,
      parameters: (json['parameters'] as List<Object?>).map((item) => RuntimeHostInteractionToolPermissionToolParameter.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'parameters': parameters.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `parameters` serialized as `parameters`.
  final List<RuntimeHostInteractionToolPermissionToolParameter> parameters;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionToolPermissionToolParameter`.
class RuntimeHostInteractionToolPermissionToolParameter {
  const RuntimeHostInteractionToolPermissionToolParameter({
    required this.name,
    required this.value,
  });

  factory RuntimeHostInteractionToolPermissionToolParameter.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionToolPermissionToolParameter(
      name: json['name'] as String,
      value: json['value'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'value': value,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `value` serialized as `value`.
  final String value;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionTtsPlaybackPayload`.
class RuntimeHostInteractionTtsPlaybackPayload {
  const RuntimeHostInteractionTtsPlaybackPayload({
    required this.command,
    required this.audioPath,
    required this.text,
    required this.voice,
    required this.locale,
    required this.speed,
    required this.pitch,
    required this.interrupt,
  });

  factory RuntimeHostInteractionTtsPlaybackPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionTtsPlaybackPayload(
      command: json['command'] as String,
      audioPath: json['audioPath'] == null ? null : json['audioPath'] as String,
      text: json['text'] as String,
      voice: json['voice'] as String,
      locale: json['locale'] as String,
      speed: (json['speed'] as num).toDouble(),
      pitch: (json['pitch'] as num).toDouble(),
      interrupt: json['interrupt'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'command': command,
      'audioPath': audioPath,
      'text': text,
      'voice': voice,
      'locale': locale,
      'speed': speed,
      'pitch': pitch,
      'interrupt': interrupt,
    };
  }

  /// Rust field `command` serialized as `command`.
  final String command;
  /// Rust field `audioPath` serialized as `audioPath`.
  final String? audioPath;
  /// Rust field `text` serialized as `text`.
  final String text;
  /// Rust field `voice` serialized as `voice`.
  final String voice;
  /// Rust field `locale` serialized as `locale`.
  final String locale;
  /// Rust field `speed` serialized as `speed`.
  final double speed;
  /// Rust field `pitch` serialized as `pitch`.
  final double pitch;
  /// Rust field `interrupt` serialized as `interrupt`.
  final bool interrupt;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionTtsPlaybackResponse`.
class RuntimeHostInteractionTtsPlaybackResponse {
  const RuntimeHostInteractionTtsPlaybackResponse({
    required this.path,
    required this.active,
    required this.paused,
    required this.details,
  });

  factory RuntimeHostInteractionTtsPlaybackResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionTtsPlaybackResponse(
      path: json['path'] as String,
      active: json['active'] as bool,
      paused: json['paused'] as bool,
      details: json['details'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'active': active,
      'paused': paused,
      'details': details,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `active` serialized as `active`.
  final bool active;
  /// Rust field `paused` serialized as `paused`.
  final bool paused;
  /// Rust field `details` serialized as `details`.
  final String details;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionTtsSynthesisPayload`.
class RuntimeHostInteractionTtsSynthesisPayload {
  const RuntimeHostInteractionTtsSynthesisPayload({
    required this.text,
    required this.voice,
    required this.locale,
    required this.speed,
    required this.pitch,
    required this.outputFormat,
  });

  factory RuntimeHostInteractionTtsSynthesisPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionTtsSynthesisPayload(
      text: json['text'] as String,
      voice: json['voice'] as String,
      locale: json['locale'] as String,
      speed: (json['speed'] as num).toDouble(),
      pitch: (json['pitch'] as num).toDouble(),
      outputFormat: json['outputFormat'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'text': text,
      'voice': voice,
      'locale': locale,
      'speed': speed,
      'pitch': pitch,
      'outputFormat': outputFormat,
    };
  }

  /// Rust field `text` serialized as `text`.
  final String text;
  /// Rust field `voice` serialized as `voice`.
  final String voice;
  /// Rust field `locale` serialized as `locale`.
  final String locale;
  /// Rust field `speed` serialized as `speed`.
  final double speed;
  /// Rust field `pitch` serialized as `pitch`.
  final double pitch;
  /// Rust field `outputFormat` serialized as `outputFormat`.
  final String outputFormat;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionTtsSynthesisResponse`.
class RuntimeHostInteractionTtsSynthesisResponse {
  const RuntimeHostInteractionTtsSynthesisResponse({
    required this.audioPath,
    required this.details,
  });

  factory RuntimeHostInteractionTtsSynthesisResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionTtsSynthesisResponse(
      audioPath: json['audioPath'] as String,
      details: json['details'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'audioPath': audioPath,
      'details': details,
    };
  }

  /// Rust field `audioPath` serialized as `audioPath`.
  final String audioPath;
  /// Rust field `details` serialized as `details`.
  final String details;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionWebAccessPairingPayload`.
class RuntimeHostInteractionWebAccessPairingPayload {
  const RuntimeHostInteractionWebAccessPairingPayload({
    required this.pairingId,
    required this.clientDeviceId,
    required this.clientPlatform,
    required this.clientModel,
    required this.pairingCode,
    required this.createdAt,
  });

  factory RuntimeHostInteractionWebAccessPairingPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionWebAccessPairingPayload(
      pairingId: json['pairingId'] as String,
      clientDeviceId: json['clientDeviceId'] as String,
      clientPlatform: json['clientPlatform'] as String,
      clientModel: json['clientModel'] as String,
      pairingCode: json['pairingCode'] as String,
      createdAt: json['createdAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'pairingId': pairingId,
      'clientDeviceId': clientDeviceId,
      'clientPlatform': clientPlatform,
      'clientModel': clientModel,
      'pairingCode': pairingCode,
      'createdAt': createdAt,
    };
  }

  /// Rust field `pairingId` serialized as `pairingId`.
  final String pairingId;
  /// Rust field `clientDeviceId` serialized as `clientDeviceId`.
  final String clientDeviceId;
  /// Rust field `clientPlatform` serialized as `clientPlatform`.
  final String clientPlatform;
  /// Rust field `clientModel` serialized as `clientModel`.
  final String clientModel;
  /// Rust field `pairingCode` serialized as `pairingCode`.
  final String pairingCode;
  /// Rust field `createdAt` serialized as `createdAt`.
  final int createdAt;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionWebVisitHeader`.
class RuntimeHostInteractionWebVisitHeader {
  const RuntimeHostInteractionWebVisitHeader({
    required this.name,
    required this.value,
  });

  factory RuntimeHostInteractionWebVisitHeader.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionWebVisitHeader(
      name: json['name'] as String,
      value: json['value'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'value': value,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `value` serialized as `value`.
  final String value;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionWebVisitLink`.
class RuntimeHostInteractionWebVisitLink {
  const RuntimeHostInteractionWebVisitLink({
    required this.url,
    required this.text,
  });

  factory RuntimeHostInteractionWebVisitLink.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionWebVisitLink(
      url: json['url'] as String,
      text: json['text'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'url': url,
      'text': text,
    };
  }

  /// Rust field `url` serialized as `url`.
  final String url;
  /// Rust field `text` serialized as `text`.
  final String text;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionWebVisitMetadataEntry`.
class RuntimeHostInteractionWebVisitMetadataEntry {
  const RuntimeHostInteractionWebVisitMetadataEntry({
    required this.name,
    required this.value,
  });

  factory RuntimeHostInteractionWebVisitMetadataEntry.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionWebVisitMetadataEntry(
      name: json['name'] as String,
      value: json['value'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'value': value,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `value` serialized as `value`.
  final String value;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionWebVisitPayload`.
class RuntimeHostInteractionWebVisitPayload {
  const RuntimeHostInteractionWebVisitPayload({
    required this.requestId,
    required this.url,
    required this.headers,
    required this.userAgent,
    required this.includeImageLinks,
    required this.requestedAtMillis,
  });

  factory RuntimeHostInteractionWebVisitPayload.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionWebVisitPayload(
      requestId: json['requestId'] as String,
      url: json['url'] as String,
      headers: (json['headers'] as List<Object?>).map((item) => RuntimeHostInteractionWebVisitHeader.fromJson(item as Map<String, Object?>)).toList(growable: false),
      userAgent: json['userAgent'] as String,
      includeImageLinks: json['includeImageLinks'] as bool,
      requestedAtMillis: json['requestedAtMillis'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'requestId': requestId,
      'url': url,
      'headers': headers.map((item) => item.toJson()).toList(growable: false),
      'userAgent': userAgent,
      'includeImageLinks': includeImageLinks,
      'requestedAtMillis': requestedAtMillis,
    };
  }

  /// Rust field `requestId` serialized as `requestId`.
  final String requestId;
  /// Rust field `url` serialized as `url`.
  final String url;
  /// Rust field `headers` serialized as `headers`.
  final List<RuntimeHostInteractionWebVisitHeader> headers;
  /// Rust field `userAgent` serialized as `userAgent`.
  final String userAgent;
  /// Rust field `includeImageLinks` serialized as `includeImageLinks`.
  final bool includeImageLinks;
  /// Rust field `requestedAtMillis` serialized as `requestedAtMillis`.
  final int requestedAtMillis;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionWebVisitResponse`.
class RuntimeHostInteractionWebVisitResponse {
  const RuntimeHostInteractionWebVisitResponse({
    required this.requestId,
    required this.success,
    required this.result,
    required this.error,
  });

  factory RuntimeHostInteractionWebVisitResponse.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionWebVisitResponse(
      requestId: json['requestId'] as String,
      success: json['success'] as bool,
      result: json['result'] == null ? null : RuntimeHostInteractionWebVisitResult.fromJson(json['result'] as Map<String, Object?>),
      error: json['error'] == null ? null : json['error'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'requestId': requestId,
      'success': success,
      'result': result?.toJson(),
      'error': error,
    };
  }

  /// Rust field `requestId` serialized as `requestId`.
  final String requestId;
  /// Rust field `success` serialized as `success`.
  final bool success;
  /// Rust field `result` serialized as `result`.
  final RuntimeHostInteractionWebVisitResult? result;
  /// Rust field `error` serialized as `error`.
  final String? error;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeHostInteractionService::RuntimeHostInteractionWebVisitResult`.
class RuntimeHostInteractionWebVisitResult {
  const RuntimeHostInteractionWebVisitResult({
    required this.url,
    required this.title,
    required this.content,
    required this.metadata,
    required this.links,
    required this.imageLinks,
  });

  factory RuntimeHostInteractionWebVisitResult.fromJson(Map<String, Object?> json) {
    return RuntimeHostInteractionWebVisitResult(
      url: json['url'] as String,
      title: json['title'] as String,
      content: json['content'] as String,
      metadata: (json['metadata'] as List<Object?>).map((item) => RuntimeHostInteractionWebVisitMetadataEntry.fromJson(item as Map<String, Object?>)).toList(growable: false),
      links: (json['links'] as List<Object?>).map((item) => RuntimeHostInteractionWebVisitLink.fromJson(item as Map<String, Object?>)).toList(growable: false),
      imageLinks: (json['imageLinks'] as List<Object?>).map((item) => item as String).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'url': url,
      'title': title,
      'content': content,
      'metadata': metadata.map((item) => item.toJson()).toList(growable: false),
      'links': links.map((item) => item.toJson()).toList(growable: false),
      'imageLinks': imageLinks.map((item) => item).toList(growable: false),
    };
  }

  /// Rust field `url` serialized as `url`.
  final String url;
  /// Rust field `title` serialized as `title`.
  final String title;
  /// Rust field `content` serialized as `content`.
  final String content;
  /// Rust field `metadata` serialized as `metadata`.
  final List<RuntimeHostInteractionWebVisitMetadataEntry> metadata;
  /// Rust field `links` serialized as `links`.
  final List<RuntimeHostInteractionWebVisitLink> links;
  /// Rust field `imageLinks` serialized as `imageLinks`.
  final List<String> imageLinks;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeTerminalService::RuntimeTerminalInfo`.
class RuntimeTerminalInfo {
  const RuntimeTerminalInfo({
    required this.platform,
    required this.terminal,
    required this.terminalType,
    required this.types,
  });

  factory RuntimeTerminalInfo.fromJson(Map<String, Object?> json) {
    return RuntimeTerminalInfo(
      platform: json['platform'] as String,
      terminal: json['terminal'] as String,
      terminalType: json['terminalType'] as String,
      types: (json['types'] as List<Object?>).map((item) => RuntimeTerminalTypeInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'platform': platform,
      'terminal': terminal,
      'terminalType': terminalType,
      'types': types.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `platform` serialized as `platform`.
  final String platform;
  /// Rust field `terminal` serialized as `terminal`.
  final String terminal;
  /// Rust field `terminalType` serialized as `terminalType`.
  final String terminalType;
  /// Rust field `types` serialized as `types`.
  final List<RuntimeTerminalTypeInfo> types;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeTerminalService::RuntimeTerminalScreen`.
class RuntimeTerminalScreen {
  const RuntimeTerminalScreen({
    required this.sessionId,
    required this.platform,
    required this.terminal,
    required this.terminalType,
    required this.rows,
    required this.cols,
    required this.content,
    required this.commandRunning,
  });

  factory RuntimeTerminalScreen.fromJson(Map<String, Object?> json) {
    return RuntimeTerminalScreen(
      sessionId: json['sessionId'] as String,
      platform: json['platform'] as String,
      terminal: json['terminal'] as String,
      terminalType: json['terminalType'] as String,
      rows: json['rows'] as int,
      cols: json['cols'] as int,
      content: json['content'] as String,
      commandRunning: json['commandRunning'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'platform': platform,
      'terminal': terminal,
      'terminalType': terminalType,
      'rows': rows,
      'cols': cols,
      'content': content,
      'commandRunning': commandRunning,
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `platform` serialized as `platform`.
  final String platform;
  /// Rust field `terminal` serialized as `terminal`.
  final String terminal;
  /// Rust field `terminalType` serialized as `terminalType`.
  final String terminalType;
  /// Rust field `rows` serialized as `rows`.
  final int rows;
  /// Rust field `cols` serialized as `cols`.
  final int cols;
  /// Rust field `content` serialized as `content`.
  final String content;
  /// Rust field `commandRunning` serialized as `commandRunning`.
  final bool commandRunning;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeTerminalService::RuntimeTerminalSessionInfo`.
class RuntimeTerminalSessionInfo {
  const RuntimeTerminalSessionInfo({
    required this.sessionId,
    required this.sessionName,
    required this.platform,
    required this.terminal,
    required this.terminalType,
    required this.sessionKind,
    required this.workingDir,
    required this.commandRunning,
  });

  factory RuntimeTerminalSessionInfo.fromJson(Map<String, Object?> json) {
    return RuntimeTerminalSessionInfo(
      sessionId: json['sessionId'] as String,
      sessionName: json['sessionName'] as String,
      platform: json['platform'] as String,
      terminal: json['terminal'] as String,
      terminalType: json['terminalType'] as String,
      sessionKind: json['sessionKind'] as String,
      workingDir: json['workingDir'] as String,
      commandRunning: json['commandRunning'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'sessionName': sessionName,
      'platform': platform,
      'terminal': terminal,
      'terminalType': terminalType,
      'sessionKind': sessionKind,
      'workingDir': workingDir,
      'commandRunning': commandRunning,
    };
  }

  /// Rust field `sessionId` serialized as `sessionId`.
  final String sessionId;
  /// Rust field `sessionName` serialized as `sessionName`.
  final String sessionName;
  /// Rust field `platform` serialized as `platform`.
  final String platform;
  /// Rust field `terminal` serialized as `terminal`.
  final String terminal;
  /// Rust field `terminalType` serialized as `terminalType`.
  final String terminalType;
  /// Rust field `sessionKind` serialized as `sessionKind`.
  final String sessionKind;
  /// Rust field `workingDir` serialized as `workingDir`.
  final String workingDir;
  /// Rust field `commandRunning` serialized as `commandRunning`.
  final bool commandRunning;
}

/// Generated Dart model for Rust type `operit_runtime::services::RuntimeTerminalService::RuntimeTerminalTypeInfo`.
class RuntimeTerminalTypeInfo {
  const RuntimeTerminalTypeInfo({
    required this.terminal,
    required this.terminalType,
    required this.available,
    required this.description,
  });

  factory RuntimeTerminalTypeInfo.fromJson(Map<String, Object?> json) {
    return RuntimeTerminalTypeInfo(
      terminal: json['terminal'] as String,
      terminalType: json['terminalType'] as String,
      available: json['available'] as bool,
      description: json['description'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'terminal': terminal,
      'terminalType': terminalType,
      'available': available,
      'description': description,
    };
  }

  /// Rust field `terminal` serialized as `terminal`.
  final String terminal;
  /// Rust field `terminalType` serialized as `terminalType`.
  final String terminalType;
  /// Rust field `available` serialized as `available`.
  final bool available;
  /// Rust field `description` serialized as `description`.
  final String description;
}

/// Generated Dart model for Rust type `operit_runtime::services::WorkspaceService::WorkspaceFileBytes`.
class WorkspaceFileBytes {
  const WorkspaceFileBytes({
    required this.base64Content,
  });

  factory WorkspaceFileBytes.fromJson(Map<String, Object?> json) {
    return WorkspaceFileBytes(
      base64Content: json['base64Content'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'base64Content': base64Content,
    };
  }

  /// Rust field `base64Content` serialized as `base64Content`.
  final String base64Content;
}

/// Generated Dart model for Rust type `operit_runtime::services::WorkspaceService::WorkspaceFileEntry`.
class WorkspaceFileEntry {
  const WorkspaceFileEntry({
    required this.name,
    required this.path,
    required this.relativePath,
    required this.isDirectory,
    required this.size,
    required this.lastModified,
  });

  factory WorkspaceFileEntry.fromJson(Map<String, Object?> json) {
    return WorkspaceFileEntry(
      name: json['name'] as String,
      path: json['path'] as String,
      relativePath: json['relativePath'] as String,
      isDirectory: json['isDirectory'] as bool,
      size: json['size'] as int,
      lastModified: json['lastModified'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'path': path,
      'relativePath': relativePath,
      'isDirectory': isDirectory,
      'size': size,
      'lastModified': lastModified,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `relativePath` serialized as `relativePath`.
  final String relativePath;
  /// Rust field `isDirectory` serialized as `isDirectory`.
  final bool isDirectory;
  /// Rust field `size` serialized as `size`.
  final int size;
  /// Rust field `lastModified` serialized as `lastModified`.
  final String lastModified;
}

/// Generated Dart model for Rust type `operit_runtime::services::WorkspaceService::WorkspaceManagementEntry`.
class WorkspaceManagementEntry {
  const WorkspaceManagementEntry({
    required this.name,
    required this.fullPath,
    required this.size,
  });

  factory WorkspaceManagementEntry.fromJson(Map<String, Object?> json) {
    return WorkspaceManagementEntry(
      name: json['name'] as String,
      fullPath: json['fullPath'] as String,
      size: json['size'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'fullPath': fullPath,
      'size': size,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `fullPath` serialized as `fullPath`.
  final String fullPath;
  /// Rust field `size` serialized as `size`.
  final int size;
}

/// Generated Dart model for Rust type `operit_runtime::services::WorkspaceService::WorkspaceManagementSummary`.
class WorkspaceManagementSummary {
  const WorkspaceManagementSummary({
    required this.chatHistoryCount,
    required this.boundChatCount,
    required this.workspaceRoot,
    required this.unboundWorkspaces,
  });

  factory WorkspaceManagementSummary.fromJson(Map<String, Object?> json) {
    return WorkspaceManagementSummary(
      chatHistoryCount: json['chatHistoryCount'] as int,
      boundChatCount: json['boundChatCount'] as int,
      workspaceRoot: json['workspaceRoot'] as String,
      unboundWorkspaces: (json['unboundWorkspaces'] as List<Object?>).map((item) => WorkspaceManagementEntry.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'chatHistoryCount': chatHistoryCount,
      'boundChatCount': boundChatCount,
      'workspaceRoot': workspaceRoot,
      'unboundWorkspaces': unboundWorkspaces.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `chatHistoryCount` serialized as `chatHistoryCount`.
  final int chatHistoryCount;
  /// Rust field `boundChatCount` serialized as `boundChatCount`.
  final int boundChatCount;
  /// Rust field `workspaceRoot` serialized as `workspaceRoot`.
  final String workspaceRoot;
  /// Rust field `unboundWorkspaces` serialized as `unboundWorkspaces`.
  final List<WorkspaceManagementEntry> unboundWorkspaces;
}

/// Generated Dart enum for Rust type `operit_runtime::ui::features::chat::webview::workspace::WorkspaceBackupManager::ChangeType`.
enum ChangeType {
  added('ADDED'),
  deleted('DELETED'),
  modified('MODIFIED'),
  ;

  const ChangeType(this.value);

  final String value;

  factory ChangeType.fromJson(Object? value) {
    return switch (value) {
      'ADDED' => ChangeType.added,
      'DELETED' => ChangeType.deleted,
      'MODIFIED' => ChangeType.modified,
      _ => throw ArgumentError('Unknown ChangeType: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_runtime::ui::features::chat::webview::workspace::WorkspaceBackupManager::WorkspaceFileChange`.
class WorkspaceFileChange {
  const WorkspaceFileChange({
    required this.path,
    required this.changeType,
    required this.changedLines,
  });

  factory WorkspaceFileChange.fromJson(Map<String, Object?> json) {
    return WorkspaceFileChange(
      path: json['path'] as String,
      changeType: ChangeType.fromJson(json['changeType']),
      changedLines: json['changedLines'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'path': path,
      'changeType': changeType.toJson(),
      'changedLines': changedLines,
    };
  }

  /// Rust field `path` serialized as `path`.
  final String path;
  /// Rust field `changeType` serialized as `changeType`.
  final ChangeType changeType;
  /// Rust field `changedLines` serialized as `changedLines`.
  final int changedLines;
}

/// Generated Dart model for Rust type `operit_store::repository::ChatHistoryManager::ChatImportResult`.
class ChatImportResult {
  const ChatImportResult({
    required this.newValue,
    required this.updated,
    required this.skipped,
  });

  factory ChatImportResult.fromJson(Map<String, Object?> json) {
    return ChatImportResult(
      newValue: json['new'] as int,
      updated: json['updated'] as int,
      skipped: json['skipped'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'new': newValue,
      'updated': updated,
      'skipped': skipped,
    };
  }

  /// Rust field `new` serialized as `new`.
  final int newValue;
  /// Rust field `updated` serialized as `updated`.
  final int updated;
  /// Rust field `skipped` serialized as `skipped`.
  final int skipped;
}

/// Generated Dart model for Rust type `operit_store::repository::UsageStatisticsStore::UsageRequestRecord`.
class UsageRequestRecord {
  const UsageRequestRecord({
    required this.id,
    required this.createdAtMs,
    required this.providerModel,
    required this.provider,
    required this.modelName,
    required this.functionType,
    required this.source,
    required this.chatId,
    required this.inputTokens,
    required this.outputTokens,
    required this.cachedInputTokens,
  });

  factory UsageRequestRecord.fromJson(Map<String, Object?> json) {
    return UsageRequestRecord(
      id: json['id'] as String,
      createdAtMs: json['createdAtMs'] as int,
      providerModel: json['providerModel'] as String,
      provider: json['provider'] as String,
      modelName: json['modelName'] as String,
      functionType: FunctionType.fromJson(json['functionType']),
      source: UsageRequestSource.fromJson(json['source']),
      chatId: json['chatId'] == null ? null : json['chatId'] as String,
      inputTokens: json['inputTokens'] as int,
      outputTokens: json['outputTokens'] as int,
      cachedInputTokens: json['cachedInputTokens'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'createdAtMs': createdAtMs,
      'providerModel': providerModel,
      'provider': provider,
      'modelName': modelName,
      'functionType': functionType.toJson(),
      'source': source.toJson(),
      'chatId': chatId,
      'inputTokens': inputTokens,
      'outputTokens': outputTokens,
      'cachedInputTokens': cachedInputTokens,
    };
  }

  /// Rust field `id` serialized as `id`.
  final String id;
  /// Rust field `createdAtMs` serialized as `createdAtMs`.
  final int createdAtMs;
  /// Rust field `providerModel` serialized as `providerModel`.
  final String providerModel;
  /// Rust field `provider` serialized as `provider`.
  final String provider;
  /// Rust field `modelName` serialized as `modelName`.
  final String modelName;
  /// Rust field `functionType` serialized as `functionType`.
  final FunctionType functionType;
  /// Rust field `source` serialized as `source`.
  final UsageRequestSource source;
  /// Rust field `chatId` serialized as `chatId`.
  final String? chatId;
  /// Rust field `inputTokens` serialized as `inputTokens`.
  final int inputTokens;
  /// Rust field `outputTokens` serialized as `outputTokens`.
  final int outputTokens;
  /// Rust field `cachedInputTokens` serialized as `cachedInputTokens`.
  final int cachedInputTokens;
}

/// Generated Dart enum for Rust type `operit_store::repository::UsageStatisticsStore::UsageRequestSource`.
enum UsageRequestSource {
  chatResponse('CHAT_RESPONSE'),
  toolResultResponse('TOOL_RESULT_RESPONSE'),
  summaryGeneration('SUMMARY_GENERATION'),
  memoryAnalysis('MEMORY_ANALYSIS'),
  ;

  const UsageRequestSource(this.value);

  final String value;

  factory UsageRequestSource.fromJson(Object? value) {
    return switch (value) {
      'CHAT_RESPONSE' => UsageRequestSource.chatResponse,
      'TOOL_RESULT_RESPONSE' => UsageRequestSource.toolResultResponse,
      'SUMMARY_GENERATION' => UsageRequestSource.summaryGeneration,
      'MEMORY_ANALYSIS' => UsageRequestSource.memoryAnalysis,
      _ => throw ArgumentError('Unknown UsageRequestSource: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_tools::ConversationMarkupManager::ToolResult`.
class CoreOperitToolsConversationMarkupManagerToolResult {
  const CoreOperitToolsConversationMarkupManagerToolResult({
    required this.toolName,
    required this.success,
    required this.result,
    required this.error,
  });

  factory CoreOperitToolsConversationMarkupManagerToolResult.fromJson(Map<String, Object?> json) {
    return CoreOperitToolsConversationMarkupManagerToolResult(
      toolName: json['toolName'] as String,
      success: json['success'] as bool,
      result: CoreOperitPluginSdkJsSdkResultsToolResultData.fromJson(json['result']),
      error: json['error'] == null ? null : json['error'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'toolName': toolName,
      'success': success,
      'result': result.toJson(),
      'error': error,
    };
  }

  /// Rust field `toolName` serialized as `toolName`.
  final String toolName;
  /// Rust field `success` serialized as `success`.
  final bool success;
  /// Rust field `result` serialized as `result`.
  final CoreOperitPluginSdkJsSdkResultsToolResultData result;
  /// Rust field `error` serialized as `error`.
  final String? error;
}

/// Generated Dart model for Rust type `operit_tools::ToolExecutionManager::AITool`.
class CoreOperitToolsToolExecutionManagerAiTool {
  const CoreOperitToolsToolExecutionManagerAiTool({
    required this.name,
    required this.parameters,
  });

  factory CoreOperitToolsToolExecutionManagerAiTool.fromJson(Map<String, Object?> json) {
    return CoreOperitToolsToolExecutionManagerAiTool(
      name: json['name'] as String,
      parameters: (json['parameters'] as List<Object?>).map((item) => CoreOperitToolsToolExecutionManagerToolParameter.fromJson(item as Map<String, Object?>)).toList(growable: false),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'parameters': parameters.map((item) => item.toJson()).toList(growable: false),
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `parameters` serialized as `parameters`.
  final List<CoreOperitToolsToolExecutionManagerToolParameter> parameters;
}

/// Generated Dart model for Rust type `operit_tools::ToolExecutionManager::ToolParameter`.
class CoreOperitToolsToolExecutionManagerToolParameter {
  const CoreOperitToolsToolExecutionManagerToolParameter({
    required this.name,
    required this.value,
  });

  factory CoreOperitToolsToolExecutionManagerToolParameter.fromJson(Map<String, Object?> json) {
    return CoreOperitToolsToolExecutionManagerToolParameter(
      name: json['name'] as String,
      value: json['value'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'value': value,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `value` serialized as `value`.
  final String value;
}

/// Generated Dart enum for Rust type `operit_tools::tools::AIToolHandler::ToolRegistrationVisibility`.
enum ToolRegistrationVisibility {
  public('PUBLIC'),
  internal('INTERNAL'),
  ;

  const ToolRegistrationVisibility(this.value);

  final String value;

  factory ToolRegistrationVisibility.fromJson(Object? value) {
    return switch (value) {
      'PUBLIC' => ToolRegistrationVisibility.public,
      'INTERNAL' => ToolRegistrationVisibility.internal,
      _ => throw ArgumentError('Unknown ToolRegistrationVisibility: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart enum for Rust type `operit_tools::tools::ToolPermissionSystem::AiPermissionMode`.
enum AiPermissionMode {
  readOnly('ReadOnly'),
  workspaceWrite('WorkspaceWrite'),
  full('Full'),
  ;

  const AiPermissionMode(this.value);

  final String value;

  factory AiPermissionMode.fromJson(Object? value) {
    return switch (value) {
      'ReadOnly' => AiPermissionMode.readOnly,
      'WorkspaceWrite' => AiPermissionMode.workspaceWrite,
      'Full' => AiPermissionMode.full,
      _ => throw ArgumentError('Unknown AiPermissionMode: $value'),
    };
  }

  String toJson() => value;
}

/// Generated Dart model for Rust type `operit_tools::tools::mcp_runtime::MCPLocalServer::CachedToolInfo`.
class CachedToolInfo {
  const CachedToolInfo({
    required this.name,
    required this.description,
    required this.inputSchema,
    required this.cachedAt,
  });

  factory CachedToolInfo.fromJson(Map<String, Object?> json) {
    return CachedToolInfo(
      name: json['name'] as String,
      description: json['description'] as String,
      inputSchema: json['inputSchema'] as String,
      cachedAt: json['cachedAt'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'description': description,
      'inputSchema': inputSchema,
      'cachedAt': cachedAt,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `inputSchema` serialized as `inputSchema`.
  final String inputSchema;
  /// Rust field `cachedAt` serialized as `cachedAt`.
  final int cachedAt;
}

/// Generated Dart model for Rust type `operit_tools::tools::mcp_runtime::MCPLocalServer::PluginMetadata`.
class PluginMetadata {
  const PluginMetadata({
    required this.name,
    required this.description,
    required this.author,
    required this.version,
  });

  factory PluginMetadata.fromJson(Map<String, Object?> json) {
    return PluginMetadata(
      name: json['name'] as String,
      description: json['description'] as String,
      author: json['author'] as String,
      version: json['version'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'description': description,
      'author': author,
      'version': version,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `author` serialized as `author`.
  final String author;
  /// Rust field `version` serialized as `version`.
  final String version;
}

/// Generated Dart model for Rust type `operit_tools::tools::mcp_runtime::MCPLocalServer::ServerConfig`.
class ServerConfig {
  const ServerConfig({
    required this.command,
    required this.args,
    required this.url,
    required this.type,
    required this.headers,
    required this.disabled,
    required this.autoApprove,
    required this.env,
  });

  factory ServerConfig.fromJson(Map<String, Object?> json) {
    return ServerConfig(
      command: json['command'] as String,
      args: (json['args'] as List<Object?>).map((item) => item as String).toList(growable: false),
      url: json['url'] == null ? null : json['url'] as String,
      type: json['type'] == null ? null : json['type'] as String,
      headers: (json['headers'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
      disabled: json['disabled'] as bool,
      autoApprove: (json['autoApprove'] as List<Object?>).map((item) => item as String).toList(growable: false),
      env: (json['env'] as Map<Object?, Object?>).map((key, value) => MapEntry(key as String, value as String)),
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'command': command,
      'args': args.map((item) => item).toList(growable: false),
      'url': url,
      'type': type,
      'headers': headers.map((key, value) => MapEntry(key, value)),
      'disabled': disabled,
      'autoApprove': autoApprove.map((item) => item).toList(growable: false),
      'env': env.map((key, value) => MapEntry(key, value)),
    };
  }

  /// Rust field `command` serialized as `command`.
  final String command;
  /// Rust field `args` serialized as `args`.
  final List<String> args;
  /// Rust field `url` serialized as `url`.
  final String? url;
  /// Rust field `r#type` serialized as `type`.
  final String? type;
  /// Rust field `headers` serialized as `headers`.
  final Map<String, String> headers;
  /// Rust field `disabled` serialized as `disabled`.
  final bool disabled;
  /// Rust field `autoApprove` serialized as `autoApprove`.
  final List<String> autoApprove;
  /// Rust field `env` serialized as `env`.
  final Map<String, String> env;
}

/// Generated Dart model for Rust type `operit_tools::tools::mcp_runtime::MCPLocalServer::ServerStatus`.
class ServerStatus {
  const ServerStatus({
    required this.serverId,
    required this.lastStartTime,
    required this.lastStopTime,
    required this.errorMessage,
    required this.cachedTools,
    required this.toolsCachedTime,
  });

  factory ServerStatus.fromJson(Map<String, Object?> json) {
    return ServerStatus(
      serverId: json['serverId'] as String,
      lastStartTime: json['lastStartTime'] as int,
      lastStopTime: json['lastStopTime'] as int,
      errorMessage: json['errorMessage'] == null ? null : json['errorMessage'] as String,
      cachedTools: json['cachedTools'] == null ? null : (json['cachedTools'] as List<Object?>).map((item) => CachedToolInfo.fromJson(item as Map<String, Object?>)).toList(growable: false),
      toolsCachedTime: json['toolsCachedTime'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'serverId': serverId,
      'lastStartTime': lastStartTime,
      'lastStopTime': lastStopTime,
      'errorMessage': errorMessage,
      'cachedTools': cachedTools?.map((item) => item.toJson()).toList(growable: false),
      'toolsCachedTime': toolsCachedTime,
    };
  }

  /// Rust field `serverId` serialized as `serverId`.
  final String serverId;
  /// Rust field `lastStartTime` serialized as `lastStartTime`.
  final int lastStartTime;
  /// Rust field `lastStopTime` serialized as `lastStopTime`.
  final int lastStopTime;
  /// Rust field `errorMessage` serialized as `errorMessage`.
  final String? errorMessage;
  /// Rust field `cachedTools` serialized as `cachedTools`.
  final List<CachedToolInfo>? cachedTools;
  /// Rust field `toolsCachedTime` serialized as `toolsCachedTime`.
  final int toolsCachedTime;
}

/// Generated Dart model for Rust type `operit_tools::tools::packTool::RuntimePackageManager::BundledExternalPackageCandidate`.
class BundledExternalPackageCandidate {
  const BundledExternalPackageCandidate({
    required this.packageName,
    required this.displayName,
    required this.description,
    required this.author,
    required this.packageKind,
    required this.sourcePath,
    required this.sourceFileName,
    required this.isToolPkg,
    required this.version,
    required this.category,
    required this.toolCount,
    required this.subpackageCount,
  });

  factory BundledExternalPackageCandidate.fromJson(Map<String, Object?> json) {
    return BundledExternalPackageCandidate(
      packageName: json['packageName'] as String,
      displayName: LocalizedText.fromJson(json['displayName'] as Map<String, Object?>),
      description: LocalizedText.fromJson(json['description'] as Map<String, Object?>),
      author: (json['author'] as List<Object?>).map((item) => item as String).toList(growable: false),
      packageKind: json['packageKind'] as String,
      sourcePath: json['sourcePath'] as String,
      sourceFileName: json['sourceFileName'] as String,
      isToolPkg: json['isToolPkg'] as bool,
      version: json['version'] as String,
      category: json['category'] as String,
      toolCount: json['toolCount'] as int,
      subpackageCount: json['subpackageCount'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'packageName': packageName,
      'displayName': displayName.toJson(),
      'description': description.toJson(),
      'author': author.map((item) => item).toList(growable: false),
      'packageKind': packageKind,
      'sourcePath': sourcePath,
      'sourceFileName': sourceFileName,
      'isToolPkg': isToolPkg,
      'version': version,
      'category': category,
      'toolCount': toolCount,
      'subpackageCount': subpackageCount,
    };
  }

  /// Rust field `packageName` serialized as `packageName`.
  final String packageName;
  /// Rust field `displayName` serialized as `displayName`.
  final LocalizedText displayName;
  /// Rust field `description` serialized as `description`.
  final LocalizedText description;
  /// Rust field `author` serialized as `author`.
  final List<String> author;
  /// Rust field `packageKind` serialized as `packageKind`.
  final String packageKind;
  /// Rust field `sourcePath` serialized as `sourcePath`.
  final String sourcePath;
  /// Rust field `sourceFileName` serialized as `sourceFileName`.
  final String sourceFileName;
  /// Rust field `isToolPkg` serialized as `isToolPkg`.
  final bool isToolPkg;
  /// Rust field `version` serialized as `version`.
  final String version;
  /// Rust field `category` serialized as `category`.
  final String category;
  /// Rust field `toolCount` serialized as `toolCount`.
  final int toolCount;
  /// Rust field `subpackageCount` serialized as `subpackageCount`.
  final int subpackageCount;
}

/// Generated Dart model for Rust type `operit_tools::tools::skill::SkillManager::BundledExternalSkillCandidate`.
class BundledExternalSkillCandidate {
  const BundledExternalSkillCandidate({
    required this.name,
    required this.description,
  });

  factory BundledExternalSkillCandidate.fromJson(Map<String, Object?> json) {
    return BundledExternalSkillCandidate(
      name: json['name'] as String,
      description: json['description'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'description': description,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final String description;
}

/// Generated Dart model for Rust type `operit_tools::tools::skill::SkillPackage::SkillPackage`.
class SkillPackage {
  const SkillPackage({
    required this.name,
    required this.description,
    required this.directory,
    required this.skillFile,
  });

  factory SkillPackage.fromJson(Map<String, Object?> json) {
    return SkillPackage(
      name: json['name'] as String,
      description: json['description'] as String,
      directory: json['directory'],
      skillFile: json['skillFile'],
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'description': description,
      'directory': directory,
      'skillFile': skillFile,
    };
  }

  /// Rust field `name` serialized as `name`.
  final String name;
  /// Rust field `description` serialized as `description`.
  final String description;
  /// Rust field `directory` serialized as `directory`.
  final Object? directory;
  /// Rust field `skillFile` serialized as `skillFile`.
  final Object? skillFile;
}

/// Generated Dart model for Rust type `operit_util::MarkdownRenderStream::MarkdownStreamEvent`.
class MarkdownStreamEvent {
  const MarkdownStreamEvent({
    required this.chatId,
    required this.eventType,
    required this.value,
    required this.id,
    required this.blockId,
    required this.inlineId,
    required this.parentBlockId,
    required this.nodeType,
    required this.headerLevel,
  });

  factory MarkdownStreamEvent.fromJson(Map<String, Object?> json) {
    return MarkdownStreamEvent(
      chatId: json['chatId'] as String,
      eventType: json['type'] as String,
      value: json['value'] == null ? null : json['value'] as String,
      id: json['id'] == null ? null : json['id'] as String,
      blockId: json['blockId'] == null ? null : json['blockId'] as int,
      inlineId: json['inlineId'] == null ? null : json['inlineId'] as int,
      parentBlockId: json['parentBlockId'] == null ? null : json['parentBlockId'] as int,
      nodeType: json['nodeType'] == null ? null : json['nodeType'] as String,
      headerLevel: json['headerLevel'] == null ? null : json['headerLevel'] as int,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'chatId': chatId,
      'type': eventType,
      'value': value,
      'id': id,
      'blockId': blockId,
      'inlineId': inlineId,
      'parentBlockId': parentBlockId,
      'nodeType': nodeType,
      'headerLevel': headerLevel,
    };
  }

  /// Rust field `chatId` serialized as `chatId`.
  final String chatId;
  /// Rust field `eventType` serialized as `type`.
  final String eventType;
  /// Rust field `value` serialized as `value`.
  final String? value;
  /// Rust field `id` serialized as `id`.
  final String? id;
  /// Rust field `blockId` serialized as `blockId`.
  final int? blockId;
  /// Rust field `inlineId` serialized as `inlineId`.
  final int? inlineId;
  /// Rust field `parentBlockId` serialized as `parentBlockId`.
  final int? parentBlockId;
  /// Rust field `nodeType` serialized as `nodeType`.
  final String? nodeType;
  /// Rust field `headerLevel` serialized as `headerLevel`.
  final int? headerLevel;
}

