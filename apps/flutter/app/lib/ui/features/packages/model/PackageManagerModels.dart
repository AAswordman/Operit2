// ignore_for_file: file_names

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

class PackageManagerSnapshot {
  const PackageManagerSnapshot({
    required this.availablePackages,
    required this.enabledPackageNames,
    required this.pluginContainers,
    required this.enabledPluginContainerNames,
    required this.bundledExternalPackageCandidates,
  });

  factory PackageManagerSnapshot.empty() {
    return const PackageManagerSnapshot(
      availablePackages: <String, core_proxy.ToolPackage>{},
      enabledPackageNames: <String>{},
      pluginContainers: <core_proxy.ToolPkgContainerRuntime>[],
      enabledPluginContainerNames: <String>{},
      bundledExternalPackageCandidates:
          <core_proxy.BundledExternalPackageCandidate>[],
    );
  }

  final Map<String, core_proxy.ToolPackage> availablePackages;
  final Set<String> enabledPackageNames;
  final List<core_proxy.ToolPkgContainerRuntime> pluginContainers;
  final Set<String> enabledPluginContainerNames;
  final List<core_proxy.BundledExternalPackageCandidate>
  bundledExternalPackageCandidates;

  bool get isEmpty =>
      availablePackages.isEmpty &&
      pluginContainers.isEmpty &&
      bundledExternalPackageCandidates.isEmpty;

  PackageManagerSnapshot copyWith({
    Set<String>? enabledPackageNames,
    Set<String>? enabledPluginContainerNames,
  }) {
    return PackageManagerSnapshot(
      availablePackages: availablePackages,
      enabledPackageNames: enabledPackageNames ?? this.enabledPackageNames,
      pluginContainers: pluginContainers,
      enabledPluginContainerNames:
          enabledPluginContainerNames ?? this.enabledPluginContainerNames,
      bundledExternalPackageCandidates: bundledExternalPackageCandidates,
    );
  }
}
