// ignore_for_file: file_names

class HostEnvironmentDescriptor {
  const HostEnvironmentDescriptor({
    required this.id,
    required this.displayName,
    required this.pathStyleDescriptionEn,
    required this.pathStyleDescriptionCn,
    required this.examplePaths,
    required this.usesEnvironmentParameter,
    required this.environmentParameterDescriptionEn,
    required this.environmentParameterDescriptionCn,
    required this.capabilities,
    required this.fileSystemHost,
    required this.webVisitHost,
    required this.systemOperationHost,
    required this.managedRuntimeHost,
    required this.runtimeStorageHost,
    required this.runtimeSqliteHost,
  });

  factory HostEnvironmentDescriptor.fromJson(Map<String, Object?> json) {
    return HostEnvironmentDescriptor(
      id: json['id'] as String,
      displayName: json['displayName'] as String,
      pathStyleDescriptionEn: json['pathStyleDescriptionEn'] as String,
      pathStyleDescriptionCn: json['pathStyleDescriptionCn'] as String,
      examplePaths: (json['examplePaths'] as List<Object?>).cast<String>(),
      usesEnvironmentParameter: json['usesEnvironmentParameter'] as bool,
      environmentParameterDescriptionEn:
          json['environmentParameterDescriptionEn'] as String,
      environmentParameterDescriptionCn:
          json['environmentParameterDescriptionCn'] as String,
      capabilities: (json['capabilities'] as List<Object?>).cast<String>(),
      fileSystemHost: json['fileSystemHost'] as bool,
      webVisitHost: json['webVisitHost'] as bool,
      systemOperationHost: json['systemOperationHost'] as bool,
      managedRuntimeHost: json['managedRuntimeHost'] as bool,
      runtimeStorageHost: json['runtimeStorageHost'] as bool,
      runtimeSqliteHost: json['runtimeSqliteHost'] as bool,
    );
  }

  final String id;
  final String displayName;
  final String pathStyleDescriptionEn;
  final String pathStyleDescriptionCn;
  final List<String> examplePaths;
  final bool usesEnvironmentParameter;
  final String environmentParameterDescriptionEn;
  final String environmentParameterDescriptionCn;
  final List<String> capabilities;
  final bool fileSystemHost;
  final bool webVisitHost;
  final bool systemOperationHost;
  final bool managedRuntimeHost;
  final bool runtimeStorageHost;
  final bool runtimeSqliteHost;
}
