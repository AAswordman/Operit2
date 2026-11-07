// GENERATED FILE. Source: operit-proxy-scan.

package operit.plugin.sdk

/** Generated client for Core object `application`. */
class OperitApplicationClient(private val client: OperitPluginSdkClient) {
    /** Watches `pluginLoadingProgressFlow` through the Core Link route. */
    fun pluginLoadingProgressFlow(): kotlinx.coroutines.flow.Flow<PluginLoadingProgress> = client.watchTyped("core/application", "pluginLoadingProgressFlow", mapOf<String, Any?>(), ::decodePluginLoadingProgress)
    /** Calls `getExtensionCatalogSnapshot` through the Core Link route. */
    suspend fun getExtensionCatalogSnapshot(): ExtensionCatalogSnapshot = client.callTyped("core/application", "getExtensionCatalogSnapshot", mapOf<String, Any?>(), ::decodeExtensionCatalogSnapshot)
}

/** Generated client for Core object `application.packageManager`. */
class OperitApplicationPackageManagerClient(private val client: OperitPluginSdkClient) {
    /** Calls `activatePackage` through the Core Link route. */
    suspend fun activatePackage(packageName: String): Boolean = client.callTyped("core/application.packageManager", "activatePackage", mapOf<String, Any?>("packageName" to packageName), ::decodeBoolean)
    /** Calls `isPackageActivated` through the Core Link route. */
    suspend fun isPackageActivated(packageName: String): Boolean = client.callTyped("core/application.packageManager", "isPackageActivated", mapOf<String, Any?>("packageName" to packageName), ::decodeBoolean)
    /** Calls `usePackage` through the Core Link route. */
    suspend fun usePackage(packageName: String): String = client.callTyped("core/application.packageManager", "usePackage", mapOf<String, Any?>("packageName" to packageName), ::decodeString)
    /** Calls `executeUsePackageTool` through the Core Link route. */
    suspend fun executeUsePackageTool(toolName: String, packageName: String): ToolResult = client.callTyped("core/application.packageManager", "executeUsePackageTool", mapOf<String, Any?>("toolName" to toolName, "packageName" to packageName), ::decodeToolResult)
    /** Calls `getEnabledPackageNames` through the Core Link route. */
    suspend fun getEnabledPackageNames(): List<String> = client.callTyped("core/application.packageManager", "getEnabledPackageNames", mapOf<String, Any?>(), ::decodeListString)
    /** Calls `isPackageEnabled` through the Core Link route. */
    suspend fun isPackageEnabled(packageName: String): Boolean = client.callTyped("core/application.packageManager", "isPackageEnabled", mapOf<String, Any?>("packageName" to packageName), ::decodeBoolean)
    /** Calls `getActivePackageNames` through the Core Link route. */
    suspend fun getActivePackageNames(): List<String> = client.callTyped("core/application.packageManager", "getActivePackageNames", mapOf<String, Any?>(), ::decodeListString)
    /** Calls `enablePackage` through the Core Link route. */
    suspend fun enablePackage(packageName: String): String = client.callTyped("core/application.packageManager", "enablePackage", mapOf<String, Any?>("packageName" to packageName), ::decodeString)
    /** Calls `disablePackage` through the Core Link route. */
    suspend fun disablePackage(packageName: String): String = client.callTyped("core/application.packageManager", "disablePackage", mapOf<String, Any?>("packageName" to packageName), ::decodeString)
    /** Calls `getToolPkgPluginContainerDetails` through the Core Link route. */
    suspend fun getToolPkgPluginContainerDetails(useEnglish: Boolean): List<ToolPkgContainerDetails> = client.callTyped("core/application.packageManager", "getToolPkgPluginContainerDetails", mapOf<String, Any?>("useEnglish" to useEnglish), ::decodeListToolPkgContainerDetails)
    /** Calls `getToolPkgContainerRuntimes` through the Core Link route. */
    suspend fun getToolPkgContainerRuntimes(): List<ToolPkgContainerRuntime> = client.callTyped("core/application.packageManager", "getToolPkgContainerRuntimes", mapOf<String, Any?>(), ::decodeListToolPkgContainerRuntime)
    /** Calls `getToolPkgContainerDetails` through the Core Link route. */
    suspend fun getToolPkgContainerDetails(packageName: String, useEnglish: Boolean): ToolPkgContainerDetails? = client.callTyped("core/application.packageManager", "getToolPkgContainerDetails", mapOf<String, Any?>("packageName" to packageName, "useEnglish" to useEnglish), ::decodeNullableToolPkgContainerDetails)
    /** Calls `readToolPkgLogoBytes` through the Core Link route. */
    suspend fun readToolPkgLogoBytes(packageName: String): ToolPkgLogoBytes? = client.callTyped("core/application.packageManager", "readToolPkgLogoBytes", mapOf<String, Any?>("packageName" to packageName), ::decodeNullableToolPkgLogoBytes)
    /** Calls `getEffectivePackageTools` through the Core Link route. */
    suspend fun getEffectivePackageTools(packageName: String): ToolPackage? = client.callTyped("core/application.packageManager", "getEffectivePackageTools", mapOf<String, Any?>("packageName" to packageName), ::decodeNullableToolPackage)
    /** Calls `getPackageTools` through the Core Link route. */
    suspend fun getPackageTools(packageName: String): ToolPackage? = client.callTyped("core/application.packageManager", "getPackageTools", mapOf<String, Any?>("packageName" to packageName), ::decodeNullableToolPackage)
    /** Calls `getAvailablePackages` through the Core Link route. */
    suspend fun getAvailablePackages(): Map<String, ToolPackage> = client.callTyped("core/application.packageManager", "getAvailablePackages", mapOf<String, Any?>(), ::decodeMapStringToolPackage)
    /** Calls `getToolPkgLoadIssues` through the Core Link route. */
    suspend fun getToolPkgLoadIssues(): List<ToolPkgLoadIssue> = client.callTyped("core/application.packageManager", "getToolPkgLoadIssues", mapOf<String, Any?>(), ::decodeListToolPkgLoadIssue)
}

