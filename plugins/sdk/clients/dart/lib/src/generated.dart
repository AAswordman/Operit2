// GENERATED FILE. Source: operit-proxy-scan.

import 'client.dart';
import 'models.dart';

/// Generated client for Core object `application`.
final class OperitApplicationClient {
  const OperitApplicationClient(this._client);
  final OperitPluginSdkClient _client;
  /// Watches `pluginLoadingProgressFlow` through the Core Link route.
  Stream<PluginLoadingProgress> pluginLoadingProgressFlow() => _client.watchTyped<PluginLoadingProgress>("core/application", 'pluginLoadingProgressFlow', <String, Object?>{}, (value) => PluginLoadingProgress.fromMessagePackValue(value as Map<String, Object?>));
  /// Calls `getExtensionCatalogSnapshot` through the Core Link route.
  Future<ExtensionCatalogSnapshot> getExtensionCatalogSnapshot() async { return await _client.callTyped<ExtensionCatalogSnapshot>("core/application", 'getExtensionCatalogSnapshot', <String, Object?>{}, (value) => ExtensionCatalogSnapshot.fromMessagePackValue(value as Map<String, Object?>)); }
}

/// Generated client for Core object `application.packageManager`.
final class OperitApplicationPackageManagerClient {
  const OperitApplicationPackageManagerClient(this._client);
  final OperitPluginSdkClient _client;
  /// Calls `activatePackage` through the Core Link route.
  Future<bool> activatePackage({required String packageName}) async { return await _client.callTyped<bool>("core/application.packageManager", 'activatePackage', <String, Object?>{'packageName': packageName}, (value) => value as bool); }
  /// Calls `isPackageActivated` through the Core Link route.
  Future<bool> isPackageActivated({required String packageName}) async { return await _client.callTyped<bool>("core/application.packageManager", 'isPackageActivated', <String, Object?>{'packageName': packageName}, (value) => value as bool); }
  /// Calls `usePackage` through the Core Link route.
  Future<String> usePackage({required String packageName}) async { return await _client.callTyped<String>("core/application.packageManager", 'usePackage', <String, Object?>{'packageName': packageName}, (value) => value as String); }
  /// Calls `executeUsePackageTool` through the Core Link route.
  Future<ToolResult> executeUsePackageTool({required String toolName, required String packageName}) async { return await _client.callTyped<ToolResult>("core/application.packageManager", 'executeUsePackageTool', <String, Object?>{'toolName': toolName, 'packageName': packageName}, (value) => ToolResult.fromMessagePackValue(value as Map<String, Object?>)); }
  /// Calls `getEnabledPackageNames` through the Core Link route.
  Future<List<String>> getEnabledPackageNames() async { return await _client.callTyped<List<String>>("core/application.packageManager", 'getEnabledPackageNames', <String, Object?>{}, (value) => (value as List<Object?>).map((item) => item as String).toList(growable: false)); }
  /// Calls `isPackageEnabled` through the Core Link route.
  Future<bool> isPackageEnabled({required String packageName}) async { return await _client.callTyped<bool>("core/application.packageManager", 'isPackageEnabled', <String, Object?>{'packageName': packageName}, (value) => value as bool); }
  /// Calls `getActivePackageNames` through the Core Link route.
  Future<List<String>> getActivePackageNames() async { return await _client.callTyped<List<String>>("core/application.packageManager", 'getActivePackageNames', <String, Object?>{}, (value) => (value as List<Object?>).map((item) => item as String).toList(growable: false)); }
  /// Calls `enablePackage` through the Core Link route.
  Future<String> enablePackage({required String packageName}) async { return await _client.callTyped<String>("core/application.packageManager", 'enablePackage', <String, Object?>{'packageName': packageName}, (value) => value as String); }
  /// Calls `disablePackage` through the Core Link route.
  Future<String> disablePackage({required String packageName}) async { return await _client.callTyped<String>("core/application.packageManager", 'disablePackage', <String, Object?>{'packageName': packageName}, (value) => value as String); }
  /// Calls `getToolPkgPluginContainerDetails` through the Core Link route.
  Future<List<ToolPkgContainerDetails>> getToolPkgPluginContainerDetails({required bool useEnglish}) async { return await _client.callTyped<List<ToolPkgContainerDetails>>("core/application.packageManager", 'getToolPkgPluginContainerDetails', <String, Object?>{'useEnglish': useEnglish}, (value) => (value as List<Object?>).map((item) => ToolPkgContainerDetails.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false)); }
  /// Calls `getToolPkgContainerRuntimes` through the Core Link route.
  Future<List<ToolPkgContainerRuntime>> getToolPkgContainerRuntimes() async { return await _client.callTyped<List<ToolPkgContainerRuntime>>("core/application.packageManager", 'getToolPkgContainerRuntimes', <String, Object?>{}, (value) => (value as List<Object?>).map((item) => ToolPkgContainerRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false)); }
  /// Calls `getToolPkgContainerDetails` through the Core Link route.
  Future<ToolPkgContainerDetails?> getToolPkgContainerDetails({required String packageName, required bool useEnglish}) async { return await _client.callTyped<ToolPkgContainerDetails?>("core/application.packageManager", 'getToolPkgContainerDetails', <String, Object?>{'packageName': packageName, 'useEnglish': useEnglish}, (value) => value == null ? null : ToolPkgContainerDetails.fromMessagePackValue(value as Map<String, Object?>)); }
  /// Calls `readToolPkgLogoBytes` through the Core Link route.
  Future<ToolPkgLogoBytes?> readToolPkgLogoBytes({required String packageName}) async { return await _client.callTyped<ToolPkgLogoBytes?>("core/application.packageManager", 'readToolPkgLogoBytes', <String, Object?>{'packageName': packageName}, (value) => value == null ? null : ToolPkgLogoBytes.fromMessagePackValue(value as Map<String, Object?>)); }
  /// Calls `getEffectivePackageTools` through the Core Link route.
  Future<ToolPackage?> getEffectivePackageTools({required String packageName}) async { return await _client.callTyped<ToolPackage?>("core/application.packageManager", 'getEffectivePackageTools', <String, Object?>{'packageName': packageName}, (value) => value == null ? null : ToolPackage.fromMessagePackValue(value as Map<String, Object?>)); }
  /// Calls `getPackageTools` through the Core Link route.
  Future<ToolPackage?> getPackageTools({required String packageName}) async { return await _client.callTyped<ToolPackage?>("core/application.packageManager", 'getPackageTools', <String, Object?>{'packageName': packageName}, (value) => value == null ? null : ToolPackage.fromMessagePackValue(value as Map<String, Object?>)); }
  /// Calls `getAvailablePackages` through the Core Link route.
  Future<Map<String, ToolPackage>> getAvailablePackages() async { return await _client.callTyped<Map<String, ToolPackage>>("core/application.packageManager", 'getAvailablePackages', <String, Object?>{}, (value) => (value as Map).map((key, item) => MapEntry(key as String, ToolPackage.fromMessagePackValue(item as Map<String, Object?>)))); }
  /// Calls `getToolPkgLoadIssues` through the Core Link route.
  Future<List<ToolPkgLoadIssue>> getToolPkgLoadIssues() async { return await _client.callTyped<List<ToolPkgLoadIssue>>("core/application.packageManager", 'getToolPkgLoadIssues', <String, Object?>{}, (value) => (value as List<Object?>).map((item) => ToolPkgLoadIssue.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false)); }
}

