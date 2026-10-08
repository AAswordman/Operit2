// GENERATED FILE. Source: operit-proxy-scan.

package operit.plugin.sdk

fun decodeBoolean(value: Any?): Boolean = value as Boolean
fun decodeString(value: Any?): String = value as String
fun decodeInt(value: Any?): Int = (value as Number).toInt()
fun decodeDouble(value: Any?): Double = (value as Number).toDouble()
fun decodeUnit(value: Any?): Unit = Unit

fun decodeListString(value: Any?): List<String> = (value as List<*>).map { decodeString(it) }
fun decodeListToolPkgContainerDetails(value: Any?): List<ToolPkgContainerDetails> = (value as List<*>).map { decodeToolPkgContainerDetails(it) }
fun decodeListToolPkgContainerRuntime(value: Any?): List<ToolPkgContainerRuntime> = (value as List<*>).map { decodeToolPkgContainerRuntime(it) }
fun decodeListToolPkgLoadIssue(value: Any?): List<ToolPkgLoadIssue> = (value as List<*>).map { decodeToolPkgLoadIssue(it) }
/** Decodes a typed SDK map from its Link representation. */
fun decodeMapStringToolPackage(value: Any?): Map<String, ToolPackage> = (value as Map<*, *>).entries.associate { entry -> entry.key as String to decodeToolPackage(entry.value) }
fun decodeNullableToolPackage(value: Any?): ToolPackage? = value?.let { decodeToolPackage(it) }
fun decodeNullableToolPkgContainerDetails(value: Any?): ToolPkgContainerDetails? = value?.let { decodeToolPkgContainerDetails(it) }
fun decodeNullableToolPkgLogoBytes(value: Any?): ToolPkgLogoBytes? = value?.let { decodeToolPkgLogoBytes(it) }

data class EnvVar(
    val name: String,
    val description: LocalizedText,
    val required: Boolean,
    val default_value: String?
)

fun decodeEnvVar(value: Any?): EnvVar {
    val input = value as Map<*, *>
    return EnvVar(
        name = input["name"] as String as String,
        description = decodeLocalizedText(input["description"]) as LocalizedText,
        required = input["required"] as Boolean as Boolean,
        default_value = input["default_value"]?.let { it as String } as String?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun EnvVar.toMessagePackValue(): Map<String, Any?> = mapOf(
    "name" to this.name,
    "description" to this.description.toMessagePackValue(),
    "required" to this.required,
    "default_value" to this.default_value?.let { it },
)

data class LocalizedText(
    val values: Map<String, String>
)

fun decodeLocalizedText(value: Any?): LocalizedText {
    val input = value as Map<*, *>
    return LocalizedText(
        values = (input["values"] as Map<*, *>).entries.associate { entry -> entry.key as String to entry.value as String } as Map<String, String>,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun LocalizedText.toMessagePackValue(): Map<String, Any?> = mapOf(
    "values" to this.values.entries.associate { entry -> entry.key to entry.value },
)

data class PackageTool(
    val name: String,
    val description: LocalizedText,
    val parameters: List<PackageToolParameter>,
    val script: String,
    val advice: Boolean
)

fun decodePackageTool(value: Any?): PackageTool {
    val input = value as Map<*, *>
    return PackageTool(
        name = input["name"] as String as String,
        description = decodeLocalizedText(input["description"]) as LocalizedText,
        parameters = (input["parameters"] as List<*>).map { item -> decodePackageToolParameter(item) } as List<PackageToolParameter>,
        script = input["script"] as String as String,
        advice = input["advice"] as Boolean as Boolean,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun PackageTool.toMessagePackValue(): Map<String, Any?> = mapOf(
    "name" to this.name,
    "description" to this.description.toMessagePackValue(),
    "parameters" to this.parameters.map { item -> item.toMessagePackValue() },
    "script" to this.script,
    "advice" to this.advice,
)

data class PackageToolParameter(
    val name: String,
    val description: LocalizedText,
    val parameter_type: String,
    val required: Boolean
)

fun decodePackageToolParameter(value: Any?): PackageToolParameter {
    val input = value as Map<*, *>
    return PackageToolParameter(
        name = input["name"] as String as String,
        description = decodeLocalizedText(input["description"]) as LocalizedText,
        parameter_type = input["parameter_type"] as String as String,
        required = input["required"] as Boolean as Boolean,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun PackageToolParameter.toMessagePackValue(): Map<String, Any?> = mapOf(
    "name" to this.name,
    "description" to this.description.toMessagePackValue(),
    "parameter_type" to this.parameter_type,
    "required" to this.required,
)

data class ToolPackage(
    val name: String,
    val description: LocalizedText,
    val tools: List<PackageTool>,
    val states: List<ToolPackageState>,
    val env: List<EnvVar>,
    val is_built_in: Boolean,
    val enabled_by_default: Boolean,
    val display_name: LocalizedText,
    val category: String,
    val author: List<String>
)

fun decodeToolPackage(value: Any?): ToolPackage {
    val input = value as Map<*, *>
    return ToolPackage(
        name = input["name"] as String as String,
        description = decodeLocalizedText(input["description"]) as LocalizedText,
        tools = (input["tools"] as List<*>).map { item -> decodePackageTool(item) } as List<PackageTool>,
        states = (input["states"] as List<*>).map { item -> decodeToolPackageState(item) } as List<ToolPackageState>,
        env = (input["env"] as List<*>).map { item -> decodeEnvVar(item) } as List<EnvVar>,
        is_built_in = input["is_built_in"] as Boolean as Boolean,
        enabled_by_default = input["enabled_by_default"] as Boolean as Boolean,
        display_name = decodeLocalizedText(input["display_name"]) as LocalizedText,
        category = input["category"] as String as String,
        author = (input["author"] as List<*>).map { item -> item as String } as List<String>,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPackage.toMessagePackValue(): Map<String, Any?> = mapOf(
    "name" to this.name,
    "description" to this.description.toMessagePackValue(),
    "tools" to this.tools.map { item -> item.toMessagePackValue() },
    "states" to this.states.map { item -> item.toMessagePackValue() },
    "env" to this.env.map { item -> item.toMessagePackValue() },
    "is_built_in" to this.is_built_in,
    "enabled_by_default" to this.enabled_by_default,
    "display_name" to this.display_name.toMessagePackValue(),
    "category" to this.category,
    "author" to this.author.map { item -> item },
)

data class ToolPackageState(
    val id: String,
    val condition: String,
    val inherit_tools: Boolean,
    val exclude_tools: List<String>,
    val tools: List<PackageTool>
)

fun decodeToolPackageState(value: Any?): ToolPackageState {
    val input = value as Map<*, *>
    return ToolPackageState(
        id = input["id"] as String as String,
        condition = input["condition"] as String as String,
        inherit_tools = input["inherit_tools"] as Boolean as Boolean,
        exclude_tools = (input["exclude_tools"] as List<*>).map { item -> item as String } as List<String>,
        tools = (input["tools"] as List<*>).map { item -> decodePackageTool(item) } as List<PackageTool>,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPackageState.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "condition" to this.condition,
    "inherit_tools" to this.inherit_tools,
    "exclude_tools" to this.exclude_tools.map { item -> item },
    "tools" to this.tools.map { item -> item.toMessagePackValue() },
)

data class ToolPkgContainerDetails(
    val packageName: String,
    val displayName: String,
    val description: String,
    val version: String,
    val apiVersion: String,
    val logoResourceKey: String?,
    val logoMimeType: String?,
    val author: List<String>,
    val requires: List<ToolPkgManifestRequirement>,
    val resourceCount: Int,
    val workspaceTemplateCount: Int,
    val uiModuleCount: Int,
    val toolboxUiModules: List<ToolPkgToolboxUiModule>,
    val subpackages: List<ToolPkgSubpackageInfo>,
    val workspaceTemplates: List<ToolPkgWorkspaceTemplate>
)

fun decodeToolPkgContainerDetails(value: Any?): ToolPkgContainerDetails {
    val input = value as Map<*, *>
    return ToolPkgContainerDetails(
        packageName = input["packageName"] as String as String,
        displayName = input["displayName"] as String as String,
        description = input["description"] as String as String,
        version = input["version"] as String as String,
        apiVersion = input["apiVersion"] as String as String,
        logoResourceKey = input["logoResourceKey"]?.let { it as String } as String?,
        logoMimeType = input["logoMimeType"]?.let { it as String } as String?,
        author = (input["author"] as List<*>).map { item -> item as String } as List<String>,
        requires = (input["requires"] as List<*>).map { item -> decodeToolPkgManifestRequirement(item) } as List<ToolPkgManifestRequirement>,
        resourceCount = (input["resourceCount"] as Number).toInt() as Int,
        workspaceTemplateCount = (input["workspaceTemplateCount"] as Number).toInt() as Int,
        uiModuleCount = (input["uiModuleCount"] as Number).toInt() as Int,
        toolboxUiModules = (input["toolboxUiModules"] as List<*>).map { item -> decodeToolPkgToolboxUiModule(item) } as List<ToolPkgToolboxUiModule>,
        subpackages = (input["subpackages"] as List<*>).map { item -> decodeToolPkgSubpackageInfo(item) } as List<ToolPkgSubpackageInfo>,
        workspaceTemplates = (input["workspaceTemplates"] as List<*>).map { item -> decodeToolPkgWorkspaceTemplate(item) } as List<ToolPkgWorkspaceTemplate>,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgContainerDetails.toMessagePackValue(): Map<String, Any?> = mapOf(
    "packageName" to this.packageName,
    "displayName" to this.displayName,
    "description" to this.description,
    "version" to this.version,
    "apiVersion" to this.apiVersion,
    "logoResourceKey" to this.logoResourceKey?.let { it },
    "logoMimeType" to this.logoMimeType?.let { it },
    "author" to this.author.map { item -> item },
    "requires" to this.requires.map { item -> item.toMessagePackValue() },
    "resourceCount" to this.resourceCount,
    "workspaceTemplateCount" to this.workspaceTemplateCount,
    "uiModuleCount" to this.uiModuleCount,
    "toolboxUiModules" to this.toolboxUiModules.map { item -> item.toMessagePackValue() },
    "subpackages" to this.subpackages.map { item -> item.toMessagePackValue() },
    "workspaceTemplates" to this.workspaceTemplates.map { item -> item.toMessagePackValue() },
)

data class ToolPkgLogoBytes(
    val resourceKey: String,
    val mimeType: String,
    val fileName: String,
    val bytes: ByteArray
)

fun decodeToolPkgLogoBytes(value: Any?): ToolPkgLogoBytes {
    val input = value as Map<*, *>
    return ToolPkgLogoBytes(
        resourceKey = input["resourceKey"] as String as String,
        mimeType = input["mimeType"] as String as String,
        fileName = input["fileName"] as String as String,
        bytes = (input["bytes"] as List<*>).map { (it as Number).toByte() }.toByteArray() as ByteArray,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgLogoBytes.toMessagePackValue(): Map<String, Any?> = mapOf(
    "resourceKey" to this.resourceKey,
    "mimeType" to this.mimeType,
    "fileName" to this.fileName,
    "bytes" to this.bytes.map { it.toInt() and 255 },
)

data class ToolPkgSubpackageInfo(
    val packageName: String,
    val subpackageId: String,
    val displayName: String,
    val description: String,
    val enabledByDefault: Boolean,
    val toolCount: Int,
    val enabled: Boolean
)

fun decodeToolPkgSubpackageInfo(value: Any?): ToolPkgSubpackageInfo {
    val input = value as Map<*, *>
    return ToolPkgSubpackageInfo(
        packageName = input["packageName"] as String as String,
        subpackageId = input["subpackageId"] as String as String,
        displayName = input["displayName"] as String as String,
        description = input["description"] as String as String,
        enabledByDefault = input["enabledByDefault"] as Boolean as Boolean,
        toolCount = (input["toolCount"] as Number).toInt() as Int,
        enabled = input["enabled"] as Boolean as Boolean,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgSubpackageInfo.toMessagePackValue(): Map<String, Any?> = mapOf(
    "packageName" to this.packageName,
    "subpackageId" to this.subpackageId,
    "displayName" to this.displayName,
    "description" to this.description,
    "enabledByDefault" to this.enabledByDefault,
    "toolCount" to this.toolCount,
    "enabled" to this.enabled,
)

data class ToolPkgToolboxUiModule(
    val containerPackageName: String,
    val toolPkgId: String,
    val routeId: String,
    val uiModuleId: String,
    val runtime: String,
    val screen: String,
    val title: String,
    val description: String,
    val moduleSpec: Map<String, Any?>,
    val keepAlive: Boolean
)

fun decodeToolPkgToolboxUiModule(value: Any?): ToolPkgToolboxUiModule {
    val input = value as Map<*, *>
    return ToolPkgToolboxUiModule(
        containerPackageName = input["containerPackageName"] as String as String,
        toolPkgId = input["toolPkgId"] as String as String,
        routeId = input["routeId"] as String as String,
        uiModuleId = input["uiModuleId"] as String as String,
        runtime = input["runtime"] as String as String,
        screen = input["screen"] as String as String,
        title = input["title"] as String as String,
        description = input["description"] as String as String,
        moduleSpec = (input["moduleSpec"] as Map<*, *>).entries.associate { entry -> entry.key as String to entry.value } as Map<String, Any?>,
        keepAlive = input["keepAlive"] as Boolean as Boolean,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgToolboxUiModule.toMessagePackValue(): Map<String, Any?> = mapOf(
    "containerPackageName" to this.containerPackageName,
    "toolPkgId" to this.toolPkgId,
    "routeId" to this.routeId,
    "uiModuleId" to this.uiModuleId,
    "runtime" to this.runtime,
    "screen" to this.screen,
    "title" to this.title,
    "description" to this.description,
    "moduleSpec" to this.moduleSpec.entries.associate { entry -> entry.key to entry.value },
    "keepAlive" to this.keepAlive,
)

data class ToolPkgWorkspaceTemplate(
    val containerPackageName: String,
    val toolPkgId: String,
    val templateId: String,
    val displayName: String,
    val description: String,
    val resourceKey: String,
    val projectType: String
)

fun decodeToolPkgWorkspaceTemplate(value: Any?): ToolPkgWorkspaceTemplate {
    val input = value as Map<*, *>
    return ToolPkgWorkspaceTemplate(
        containerPackageName = input["containerPackageName"] as String as String,
        toolPkgId = input["toolPkgId"] as String as String,
        templateId = input["templateId"] as String as String,
        displayName = input["displayName"] as String as String,
        description = input["description"] as String as String,
        resourceKey = input["resourceKey"] as String as String,
        projectType = input["projectType"] as String as String,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgWorkspaceTemplate.toMessagePackValue(): Map<String, Any?> = mapOf(
    "containerPackageName" to this.containerPackageName,
    "toolPkgId" to this.toolPkgId,
    "templateId" to this.templateId,
    "displayName" to this.displayName,
    "description" to this.description,
    "resourceKey" to this.resourceKey,
    "projectType" to this.projectType,
)

data class ToolPkgAiProviderHandlerRuntime(
    val function: String,
    val functionSource: String?
)

fun decodeToolPkgAiProviderHandlerRuntime(value: Any?): ToolPkgAiProviderHandlerRuntime {
    val input = value as Map<*, *>
    return ToolPkgAiProviderHandlerRuntime(
        function = input["function"] as String as String,
        functionSource = input["functionSource"]?.let { it as String } as String?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgAiProviderHandlerRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "function" to this.function,
    "functionSource" to this.functionSource?.let { it },
)

data class ToolPkgAiProviderRuntime(
    val id: String,
    val displayName: String,
    val description: String,
    val listModelsHandler: ToolPkgAiProviderHandlerRuntime,
    val sendMessageHandler: ToolPkgAiProviderHandlerRuntime,
    val testConnectionHandler: ToolPkgAiProviderHandlerRuntime,
    val calculateInputTokensHandler: ToolPkgAiProviderHandlerRuntime
)

fun decodeToolPkgAiProviderRuntime(value: Any?): ToolPkgAiProviderRuntime {
    val input = value as Map<*, *>
    return ToolPkgAiProviderRuntime(
        id = input["id"] as String as String,
        displayName = input["displayName"] as String as String,
        description = input["description"] as String as String,
        listModelsHandler = decodeToolPkgAiProviderHandlerRuntime(input["listModelsHandler"]) as ToolPkgAiProviderHandlerRuntime,
        sendMessageHandler = decodeToolPkgAiProviderHandlerRuntime(input["sendMessageHandler"]) as ToolPkgAiProviderHandlerRuntime,
        testConnectionHandler = decodeToolPkgAiProviderHandlerRuntime(input["testConnectionHandler"]) as ToolPkgAiProviderHandlerRuntime,
        calculateInputTokensHandler = decodeToolPkgAiProviderHandlerRuntime(input["calculateInputTokensHandler"]) as ToolPkgAiProviderHandlerRuntime,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgAiProviderRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "displayName" to this.displayName,
    "description" to this.description,
    "listModelsHandler" to this.listModelsHandler.toMessagePackValue(),
    "sendMessageHandler" to this.sendMessageHandler.toMessagePackValue(),
    "testConnectionHandler" to this.testConnectionHandler.toMessagePackValue(),
    "calculateInputTokensHandler" to this.calculateInputTokensHandler.toMessagePackValue(),
)

data class ToolPkgAppLifecycleHookRuntime(
    val id: String,
    val event: String,
    val function: String,
    val functionSource: String?
)

fun decodeToolPkgAppLifecycleHookRuntime(value: Any?): ToolPkgAppLifecycleHookRuntime {
    val input = value as Map<*, *>
    return ToolPkgAppLifecycleHookRuntime(
        id = input["id"] as String as String,
        event = input["event"] as String as String,
        function = input["function"] as String as String,
        functionSource = input["functionSource"]?.let { it as String } as String?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgAppLifecycleHookRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "event" to this.event,
    "function" to this.function,
    "functionSource" to this.functionSource?.let { it },
)

data class ToolPkgChatComposerSlotRuntime(
    val id: String,
    val slot: String,
    val screen: String,
    val order: Int,
    val keepAlive: Boolean
)

fun decodeToolPkgChatComposerSlotRuntime(value: Any?): ToolPkgChatComposerSlotRuntime {
    val input = value as Map<*, *>
    return ToolPkgChatComposerSlotRuntime(
        id = input["id"] as String as String,
        slot = input["slot"] as String as String,
        screen = input["screen"] as String as String,
        order = (input["order"] as Number).toInt() as Int,
        keepAlive = input["keepAlive"] as Boolean as Boolean,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgChatComposerSlotRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "slot" to this.slot,
    "screen" to this.screen,
    "order" to this.order,
    "keepAlive" to this.keepAlive,
)

data class ToolPkgChatMessageMenuDialogRuntime(
    val screen: String,
    val title: LocalizedText
)

fun decodeToolPkgChatMessageMenuDialogRuntime(value: Any?): ToolPkgChatMessageMenuDialogRuntime {
    val input = value as Map<*, *>
    return ToolPkgChatMessageMenuDialogRuntime(
        screen = input["screen"] as String as String,
        title = decodeLocalizedText(input["title"]) as LocalizedText,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgChatMessageMenuDialogRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "screen" to this.screen,
    "title" to this.title.toMessagePackValue(),
)

data class ToolPkgChatMessageMenuItemRuntime(
    val id: String,
    val title: LocalizedText,
    val icon: String?,
    val order: Int,
    val senders: List<String>,
    val function: String,
    val functionSource: String?,
    val dialog: ToolPkgChatMessageMenuDialogRuntime?
)

fun decodeToolPkgChatMessageMenuItemRuntime(value: Any?): ToolPkgChatMessageMenuItemRuntime {
    val input = value as Map<*, *>
    return ToolPkgChatMessageMenuItemRuntime(
        id = input["id"] as String as String,
        title = decodeLocalizedText(input["title"]) as LocalizedText,
        icon = input["icon"]?.let { it as String } as String?,
        order = (input["order"] as Number).toInt() as Int,
        senders = (input["senders"] as List<*>).map { item -> item as String } as List<String>,
        function = input["function"] as String as String,
        functionSource = input["functionSource"]?.let { it as String } as String?,
        dialog = input["dialog"]?.let { decodeToolPkgChatMessageMenuDialogRuntime(it) } as ToolPkgChatMessageMenuDialogRuntime?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgChatMessageMenuItemRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "title" to this.title.toMessagePackValue(),
    "icon" to this.icon?.let { it },
    "order" to this.order,
    "senders" to this.senders.map { item -> item },
    "function" to this.function,
    "functionSource" to this.functionSource?.let { it },
    "dialog" to this.dialog?.let { it.toMessagePackValue() },
)

data class ToolPkgContainerRuntime(
    val packageName: String,
    val displayName: LocalizedText,
    val description: LocalizedText,
    val version: String,
    val apiVersion: String,
    val publicApi: String?,
    val publicApis: List<ToolPkgRegisteredFunctionHook>,
    val requires: List<ToolPkgManifestRequirement>,
    val dependencyIssues: List<ToolPkgDependencyIssue>,
    val manifestExtensions: Map<String, Any?>,
    val author: List<String>,
    val mainEntry: String,
    val sourceType: ToolPkgSourceType,
    val sourcePath: String,
    val subpackages: List<ToolPkgSubpackageRuntime>,
    val resources: List<ToolPkgResourceRuntime>,
    val wasmModules: List<ToolPkgWasmModuleRuntime>,
    val workflowTemplates: List<ToolPkgWorkflowTemplateRuntime>,
    val workspaceTemplates: List<ToolPkgWorkspaceTemplateRuntime>,
    val uiModules: List<ToolPkgUiModuleRuntime>,
    val uiRoutes: List<ToolPkgUiRouteRuntime>,
    val chatComposerSlots: List<ToolPkgChatComposerSlotRuntime>,
    val navigationEntries: List<ToolPkgNavigationEntryRuntime>,
    val desktopWidgets: List<ToolPkgDesktopWidgetRuntime>,
    val appLifecycleHooks: List<ToolPkgAppLifecycleHookRuntime>,
    val messageProcessingPlugins: List<ToolPkgFunctionHookRuntime>,
    val xmlRenderPlugins: List<ToolPkgTagFunctionHookRuntime>,
    val inputMenuTogglePlugins: List<ToolPkgFunctionHookRuntime>,
    val chatInputHooks: List<ToolPkgFunctionHookRuntime>,
    val chatViewHooks: List<ToolPkgFunctionHookRuntime>,
    val chatMessageHooks: List<ToolPkgFunctionHookRuntime>,
    val chatLifecycleHooks: List<ToolPkgFunctionHookRuntime>,
    val chatMessageMenuItems: List<ToolPkgChatMessageMenuItemRuntime>,
    val chatRuntimeHooks: List<ToolPkgFunctionHookRuntime>,
    val hostEventHooks: List<ToolPkgHostEventHookRuntime>,
    val toolLifecycleHooks: List<ToolPkgFunctionHookRuntime>,
    val promptInputHooks: List<ToolPkgFunctionHookRuntime>,
    val promptHistoryHooks: List<ToolPkgFunctionHookRuntime>,
    val promptEstimateHistoryHooks: List<ToolPkgFunctionHookRuntime>,
    val systemPromptComposeHooks: List<ToolPkgFunctionHookRuntime>,
    val toolPromptComposeHooks: List<ToolPkgFunctionHookRuntime>,
    val promptFinalizeHooks: List<ToolPkgFunctionHookRuntime>,
    val promptEstimateFinalizeHooks: List<ToolPkgFunctionHookRuntime>,
    val summaryGenerateHooks: List<ToolPkgFunctionHookRuntime>,
    val coreCommands: List<ToolPkgCoreCommandRuntime>,
    val aiProviders: List<ToolPkgAiProviderRuntime>,
    val manifestExtensionHandlers: List<ToolPkgRegisteredManifestExtension>,
    val logoResource: ToolPkgResourceRuntime?,
    val marketOrigin: ToolPkgMarketOrigin?
)

fun decodeToolPkgContainerRuntime(value: Any?): ToolPkgContainerRuntime {
    val input = value as Map<*, *>
    return ToolPkgContainerRuntime(
        packageName = input["packageName"] as String as String,
        displayName = decodeLocalizedText(input["displayName"]) as LocalizedText,
        description = decodeLocalizedText(input["description"]) as LocalizedText,
        version = input["version"] as String as String,
        apiVersion = input["apiVersion"] as String as String,
        publicApi = input["publicApi"]?.let { it as String } as String?,
        publicApis = (input["publicApis"] as List<*>).map { item -> decodeToolPkgRegisteredFunctionHook(item) } as List<ToolPkgRegisteredFunctionHook>,
        requires = (input["requires"] as List<*>).map { item -> decodeToolPkgManifestRequirement(item) } as List<ToolPkgManifestRequirement>,
        dependencyIssues = (input["dependencyIssues"] as List<*>).map { item -> decodeToolPkgDependencyIssue(item) } as List<ToolPkgDependencyIssue>,
        manifestExtensions = (input["manifestExtensions"] as Map<*, *>).entries.associate { entry -> entry.key as String to entry.value } as Map<String, Any?>,
        author = (input["author"] as List<*>).map { item -> item as String } as List<String>,
        mainEntry = input["mainEntry"] as String as String,
        sourceType = decodeToolPkgSourceType(input["sourceType"]) as ToolPkgSourceType,
        sourcePath = input["sourcePath"] as String as String,
        subpackages = (input["subpackages"] as List<*>).map { item -> decodeToolPkgSubpackageRuntime(item) } as List<ToolPkgSubpackageRuntime>,
        resources = (input["resources"] as List<*>).map { item -> decodeToolPkgResourceRuntime(item) } as List<ToolPkgResourceRuntime>,
        wasmModules = (input["wasmModules"] as List<*>).map { item -> decodeToolPkgWasmModuleRuntime(item) } as List<ToolPkgWasmModuleRuntime>,
        workflowTemplates = (input["workflowTemplates"] as List<*>).map { item -> decodeToolPkgWorkflowTemplateRuntime(item) } as List<ToolPkgWorkflowTemplateRuntime>,
        workspaceTemplates = (input["workspaceTemplates"] as List<*>).map { item -> decodeToolPkgWorkspaceTemplateRuntime(item) } as List<ToolPkgWorkspaceTemplateRuntime>,
        uiModules = (input["uiModules"] as List<*>).map { item -> decodeToolPkgUiModuleRuntime(item) } as List<ToolPkgUiModuleRuntime>,
        uiRoutes = (input["uiRoutes"] as List<*>).map { item -> decodeToolPkgUiRouteRuntime(item) } as List<ToolPkgUiRouteRuntime>,
        chatComposerSlots = (input["chatComposerSlots"] as List<*>).map { item -> decodeToolPkgChatComposerSlotRuntime(item) } as List<ToolPkgChatComposerSlotRuntime>,
        navigationEntries = (input["navigationEntries"] as List<*>).map { item -> decodeToolPkgNavigationEntryRuntime(item) } as List<ToolPkgNavigationEntryRuntime>,
        desktopWidgets = (input["desktopWidgets"] as List<*>).map { item -> decodeToolPkgDesktopWidgetRuntime(item) } as List<ToolPkgDesktopWidgetRuntime>,
        appLifecycleHooks = (input["appLifecycleHooks"] as List<*>).map { item -> decodeToolPkgAppLifecycleHookRuntime(item) } as List<ToolPkgAppLifecycleHookRuntime>,
        messageProcessingPlugins = (input["messageProcessingPlugins"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        xmlRenderPlugins = (input["xmlRenderPlugins"] as List<*>).map { item -> decodeToolPkgTagFunctionHookRuntime(item) } as List<ToolPkgTagFunctionHookRuntime>,
        inputMenuTogglePlugins = (input["inputMenuTogglePlugins"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        chatInputHooks = (input["chatInputHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        chatViewHooks = (input["chatViewHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        chatMessageHooks = (input["chatMessageHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        chatLifecycleHooks = (input["chatLifecycleHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        chatMessageMenuItems = (input["chatMessageMenuItems"] as List<*>).map { item -> decodeToolPkgChatMessageMenuItemRuntime(item) } as List<ToolPkgChatMessageMenuItemRuntime>,
        chatRuntimeHooks = (input["chatRuntimeHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        hostEventHooks = (input["hostEventHooks"] as List<*>).map { item -> decodeToolPkgHostEventHookRuntime(item) } as List<ToolPkgHostEventHookRuntime>,
        toolLifecycleHooks = (input["toolLifecycleHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        promptInputHooks = (input["promptInputHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        promptHistoryHooks = (input["promptHistoryHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        promptEstimateHistoryHooks = (input["promptEstimateHistoryHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        systemPromptComposeHooks = (input["systemPromptComposeHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        toolPromptComposeHooks = (input["toolPromptComposeHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        promptFinalizeHooks = (input["promptFinalizeHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        promptEstimateFinalizeHooks = (input["promptEstimateFinalizeHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        summaryGenerateHooks = (input["summaryGenerateHooks"] as List<*>).map { item -> decodeToolPkgFunctionHookRuntime(item) } as List<ToolPkgFunctionHookRuntime>,
        coreCommands = (input["coreCommands"] as List<*>).map { item -> decodeToolPkgCoreCommandRuntime(item) } as List<ToolPkgCoreCommandRuntime>,
        aiProviders = (input["aiProviders"] as List<*>).map { item -> decodeToolPkgAiProviderRuntime(item) } as List<ToolPkgAiProviderRuntime>,
        manifestExtensionHandlers = (input["manifestExtensionHandlers"] as List<*>).map { item -> decodeToolPkgRegisteredManifestExtension(item) } as List<ToolPkgRegisteredManifestExtension>,
        logoResource = input["logoResource"]?.let { decodeToolPkgResourceRuntime(it) } as ToolPkgResourceRuntime?,
        marketOrigin = input["marketOrigin"]?.let { decodeToolPkgMarketOrigin(it) } as ToolPkgMarketOrigin?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgContainerRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "packageName" to this.packageName,
    "displayName" to this.displayName.toMessagePackValue(),
    "description" to this.description.toMessagePackValue(),
    "version" to this.version,
    "apiVersion" to this.apiVersion,
    "publicApi" to this.publicApi?.let { it },
    "publicApis" to this.publicApis.map { item -> item.toMessagePackValue() },
    "requires" to this.requires.map { item -> item.toMessagePackValue() },
    "dependencyIssues" to this.dependencyIssues.map { item -> item.toMessagePackValue() },
    "manifestExtensions" to this.manifestExtensions.entries.associate { entry -> entry.key to entry.value },
    "author" to this.author.map { item -> item },
    "mainEntry" to this.mainEntry,
    "sourceType" to this.sourceType.toMessagePackValue(),
    "sourcePath" to this.sourcePath,
    "subpackages" to this.subpackages.map { item -> item.toMessagePackValue() },
    "resources" to this.resources.map { item -> item.toMessagePackValue() },
    "wasmModules" to this.wasmModules.map { item -> item.toMessagePackValue() },
    "workflowTemplates" to this.workflowTemplates.map { item -> item.toMessagePackValue() },
    "workspaceTemplates" to this.workspaceTemplates.map { item -> item.toMessagePackValue() },
    "uiModules" to this.uiModules.map { item -> item.toMessagePackValue() },
    "uiRoutes" to this.uiRoutes.map { item -> item.toMessagePackValue() },
    "chatComposerSlots" to this.chatComposerSlots.map { item -> item.toMessagePackValue() },
    "navigationEntries" to this.navigationEntries.map { item -> item.toMessagePackValue() },
    "desktopWidgets" to this.desktopWidgets.map { item -> item.toMessagePackValue() },
    "appLifecycleHooks" to this.appLifecycleHooks.map { item -> item.toMessagePackValue() },
    "messageProcessingPlugins" to this.messageProcessingPlugins.map { item -> item.toMessagePackValue() },
    "xmlRenderPlugins" to this.xmlRenderPlugins.map { item -> item.toMessagePackValue() },
    "inputMenuTogglePlugins" to this.inputMenuTogglePlugins.map { item -> item.toMessagePackValue() },
    "chatInputHooks" to this.chatInputHooks.map { item -> item.toMessagePackValue() },
    "chatViewHooks" to this.chatViewHooks.map { item -> item.toMessagePackValue() },
    "chatMessageHooks" to this.chatMessageHooks.map { item -> item.toMessagePackValue() },
    "chatLifecycleHooks" to this.chatLifecycleHooks.map { item -> item.toMessagePackValue() },
    "chatMessageMenuItems" to this.chatMessageMenuItems.map { item -> item.toMessagePackValue() },
    "chatRuntimeHooks" to this.chatRuntimeHooks.map { item -> item.toMessagePackValue() },
    "hostEventHooks" to this.hostEventHooks.map { item -> item.toMessagePackValue() },
    "toolLifecycleHooks" to this.toolLifecycleHooks.map { item -> item.toMessagePackValue() },
    "promptInputHooks" to this.promptInputHooks.map { item -> item.toMessagePackValue() },
    "promptHistoryHooks" to this.promptHistoryHooks.map { item -> item.toMessagePackValue() },
    "promptEstimateHistoryHooks" to this.promptEstimateHistoryHooks.map { item -> item.toMessagePackValue() },
    "systemPromptComposeHooks" to this.systemPromptComposeHooks.map { item -> item.toMessagePackValue() },
    "toolPromptComposeHooks" to this.toolPromptComposeHooks.map { item -> item.toMessagePackValue() },
    "promptFinalizeHooks" to this.promptFinalizeHooks.map { item -> item.toMessagePackValue() },
    "promptEstimateFinalizeHooks" to this.promptEstimateFinalizeHooks.map { item -> item.toMessagePackValue() },
    "summaryGenerateHooks" to this.summaryGenerateHooks.map { item -> item.toMessagePackValue() },
    "coreCommands" to this.coreCommands.map { item -> item.toMessagePackValue() },
    "aiProviders" to this.aiProviders.map { item -> item.toMessagePackValue() },
    "manifestExtensionHandlers" to this.manifestExtensionHandlers.map { item -> item.toMessagePackValue() },
    "logoResource" to this.logoResource?.let { it.toMessagePackValue() },
    "marketOrigin" to this.marketOrigin?.let { it.toMessagePackValue() },
)

data class ToolPkgCoreCommandRuntime(
    val id: String,
    val name: String,
    val title: LocalizedText,
    val description: LocalizedText,
    val usage: String,
    val function: String,
    val functionSource: String?
)

fun decodeToolPkgCoreCommandRuntime(value: Any?): ToolPkgCoreCommandRuntime {
    val input = value as Map<*, *>
    return ToolPkgCoreCommandRuntime(
        id = input["id"] as String as String,
        name = input["name"] as String as String,
        title = decodeLocalizedText(input["title"]) as LocalizedText,
        description = decodeLocalizedText(input["description"]) as LocalizedText,
        usage = input["usage"] as String as String,
        function = input["function"] as String as String,
        functionSource = input["functionSource"]?.let { it as String } as String?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgCoreCommandRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "name" to this.name,
    "title" to this.title.toMessagePackValue(),
    "description" to this.description.toMessagePackValue(),
    "usage" to this.usage,
    "function" to this.function,
    "functionSource" to this.functionSource?.let { it },
)

data class ToolPkgDependencyIssue(
    val id: String,
    val code: String,
    val requiredMinVersion: String?,
    val requiredMaxVersion: String?,
    val installedVersion: String?,
    val enabled: Boolean
)

fun decodeToolPkgDependencyIssue(value: Any?): ToolPkgDependencyIssue {
    val input = value as Map<*, *>
    return ToolPkgDependencyIssue(
        id = input["id"] as String as String,
        code = input["code"] as String as String,
        requiredMinVersion = input["requiredMinVersion"]?.let { it as String } as String?,
        requiredMaxVersion = input["requiredMaxVersion"]?.let { it as String } as String?,
        installedVersion = input["installedVersion"]?.let { it as String } as String?,
        enabled = input["enabled"] as Boolean as Boolean,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgDependencyIssue.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "code" to this.code,
    "requiredMinVersion" to this.requiredMinVersion?.let { it },
    "requiredMaxVersion" to this.requiredMaxVersion?.let { it },
    "installedVersion" to this.installedVersion?.let { it },
    "enabled" to this.enabled,
)

data class ToolPkgDesktopWidgetRuntime(
    val id: String,
    val routeId: String,
    val renderRouteId: String,
    val title: LocalizedText,
    val subtitle: LocalizedText,
    val description: LocalizedText,
    val icon: String?,
    val order: Int
)

fun decodeToolPkgDesktopWidgetRuntime(value: Any?): ToolPkgDesktopWidgetRuntime {
    val input = value as Map<*, *>
    return ToolPkgDesktopWidgetRuntime(
        id = input["id"] as String as String,
        routeId = input["routeId"] as String as String,
        renderRouteId = input["renderRouteId"] as String as String,
        title = decodeLocalizedText(input["title"]) as LocalizedText,
        subtitle = decodeLocalizedText(input["subtitle"]) as LocalizedText,
        description = decodeLocalizedText(input["description"]) as LocalizedText,
        icon = input["icon"]?.let { it as String } as String?,
        order = (input["order"] as Number).toInt() as Int,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgDesktopWidgetRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "routeId" to this.routeId,
    "renderRouteId" to this.renderRouteId,
    "title" to this.title.toMessagePackValue(),
    "subtitle" to this.subtitle.toMessagePackValue(),
    "description" to this.description.toMessagePackValue(),
    "icon" to this.icon?.let { it },
    "order" to this.order,
)

data class ToolPkgFunctionHookRuntime(
    val id: String,
    val function: String,
    val functionSource: String?
)

fun decodeToolPkgFunctionHookRuntime(value: Any?): ToolPkgFunctionHookRuntime {
    val input = value as Map<*, *>
    return ToolPkgFunctionHookRuntime(
        id = input["id"] as String as String,
        function = input["function"] as String as String,
        functionSource = input["functionSource"]?.let { it as String } as String?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgFunctionHookRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "function" to this.function,
    "functionSource" to this.functionSource?.let { it },
)

data class ToolPkgHostEventHookRuntime(
    val id: String,
    val source: String,
    val trigger: Any?,
    val function: String,
    val functionSource: String?,
    val enabled: Boolean
)

fun decodeToolPkgHostEventHookRuntime(value: Any?): ToolPkgHostEventHookRuntime {
    val input = value as Map<*, *>
    return ToolPkgHostEventHookRuntime(
        id = input["id"] as String as String,
        source = input["source"] as String as String,
        trigger = input["trigger"] as Any?,
        function = input["function"] as String as String,
        functionSource = input["functionSource"]?.let { it as String } as String?,
        enabled = input["enabled"] as Boolean as Boolean,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgHostEventHookRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "source" to this.source,
    "trigger" to this.trigger,
    "function" to this.function,
    "functionSource" to this.functionSource?.let { it },
    "enabled" to this.enabled,
)

data class ToolPkgManifestRequirement(
    val id: String,
    val description: String,
    val minVersion: String?,
    val maxVersion: String?
)

fun decodeToolPkgManifestRequirement(value: Any?): ToolPkgManifestRequirement {
    val input = value as Map<*, *>
    return ToolPkgManifestRequirement(
        id = input["id"] as String as String,
        description = input["description"] as String as String,
        minVersion = input["min_version"]?.let { it as String } as String?,
        maxVersion = input["max_version"]?.let { it as String } as String?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgManifestRequirement.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "description" to this.description,
    "min_version" to this.minVersion?.let { it },
    "max_version" to this.maxVersion?.let { it },
)

data class ToolPkgMarketOrigin(
    val market: String,
    val toolpkgId: String,
    val version: String,
    val author: List<String>
)

fun decodeToolPkgMarketOrigin(value: Any?): ToolPkgMarketOrigin {
    val input = value as Map<*, *>
    return ToolPkgMarketOrigin(
        market = input["market"] as String as String,
        toolpkgId = input["toolpkgId"] as String as String,
        version = input["version"] as String as String,
        author = (input["author"] as List<*>).map { item -> item as String } as List<String>,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgMarketOrigin.toMessagePackValue(): Map<String, Any?> = mapOf(
    "market" to this.market,
    "toolpkgId" to this.toolpkgId,
    "version" to this.version,
    "author" to this.author.map { item -> item },
)

data class ToolPkgNavigationActionHookRuntime(
    val function: String,
    val functionSource: String?
)

fun decodeToolPkgNavigationActionHookRuntime(value: Any?): ToolPkgNavigationActionHookRuntime {
    val input = value as Map<*, *>
    return ToolPkgNavigationActionHookRuntime(
        function = input["function"] as String as String,
        functionSource = input["functionSource"]?.let { it as String } as String?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgNavigationActionHookRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "function" to this.function,
    "functionSource" to this.functionSource?.let { it },
)

data class ToolPkgNavigationEntryRuntime(
    val id: String,
    val routeId: String,
    val params: Any?,
    val surface: String,
    val title: LocalizedText,
    val action: ToolPkgNavigationActionHookRuntime?,
    val icon: String?,
    val order: Int
)

fun decodeToolPkgNavigationEntryRuntime(value: Any?): ToolPkgNavigationEntryRuntime {
    val input = value as Map<*, *>
    return ToolPkgNavigationEntryRuntime(
        id = input["id"] as String as String,
        routeId = input["routeId"] as String as String,
        params = input["params"] as Any?,
        surface = input["surface"] as String as String,
        title = decodeLocalizedText(input["title"]) as LocalizedText,
        action = input["action"]?.let { decodeToolPkgNavigationActionHookRuntime(it) } as ToolPkgNavigationActionHookRuntime?,
        icon = input["icon"]?.let { it as String } as String?,
        order = (input["order"] as Number).toInt() as Int,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgNavigationEntryRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "routeId" to this.routeId,
    "params" to this.params,
    "surface" to this.surface,
    "title" to this.title.toMessagePackValue(),
    "action" to this.action?.let { it.toMessagePackValue() },
    "icon" to this.icon?.let { it },
    "order" to this.order,
)

data class ToolPkgRegisteredFunctionHook(
    val id: String,
    val function: String,
    val functionSource: String?
)

fun decodeToolPkgRegisteredFunctionHook(value: Any?): ToolPkgRegisteredFunctionHook {
    val input = value as Map<*, *>
    return ToolPkgRegisteredFunctionHook(
        id = input["id"] as String as String,
        function = input["function"] as String as String,
        functionSource = input["functionSource"]?.let { it as String } as String?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgRegisteredFunctionHook.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "function" to this.function,
    "functionSource" to this.functionSource?.let { it },
)

data class ToolPkgRegisteredManifestExtension(
    val key: String,
    val function: String,
    val functionSource: String?
)

fun decodeToolPkgRegisteredManifestExtension(value: Any?): ToolPkgRegisteredManifestExtension {
    val input = value as Map<*, *>
    return ToolPkgRegisteredManifestExtension(
        key = input["key"] as String as String,
        function = input["function"] as String as String,
        functionSource = input["functionSource"]?.let { it as String } as String?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgRegisteredManifestExtension.toMessagePackValue(): Map<String, Any?> = mapOf(
    "key" to this.key,
    "function" to this.function,
    "functionSource" to this.functionSource?.let { it },
)

data class ToolPkgResourceRuntime(
    val key: String,
    val path: String,
    val mime: String
)

fun decodeToolPkgResourceRuntime(value: Any?): ToolPkgResourceRuntime {
    val input = value as Map<*, *>
    return ToolPkgResourceRuntime(
        key = input["key"] as String as String,
        path = input["path"] as String as String,
        mime = input["mime"] as String as String,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgResourceRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "key" to this.key,
    "path" to this.path,
    "mime" to this.mime,
)

enum class ToolPkgSourceType { ASSET, MARKET, EXTERNAL }
fun decodeToolPkgSourceType(value: Any?): ToolPkgSourceType = ToolPkgSourceType.valueOf(value.toString())

/** Encodes the declared Link enum scalar. */
fun ToolPkgSourceType.toMessagePackValue(): String = when (this) {
    ToolPkgSourceType.ASSET -> "ASSET"
    ToolPkgSourceType.MARKET -> "MARKET"
    ToolPkgSourceType.EXTERNAL -> "EXTERNAL"
}

data class ToolPkgSubpackageRuntime(
    val packageName: String,
    val containerPackageName: String,
    val subpackageId: String,
    val entryPath: String,
    val displayName: LocalizedText,
    val description: LocalizedText,
    val enabledByDefault: Boolean,
    val toolCount: Int
)

fun decodeToolPkgSubpackageRuntime(value: Any?): ToolPkgSubpackageRuntime {
    val input = value as Map<*, *>
    return ToolPkgSubpackageRuntime(
        packageName = input["packageName"] as String as String,
        containerPackageName = input["containerPackageName"] as String as String,
        subpackageId = input["subpackageId"] as String as String,
        entryPath = input["entryPath"] as String as String,
        displayName = decodeLocalizedText(input["displayName"]) as LocalizedText,
        description = decodeLocalizedText(input["description"]) as LocalizedText,
        enabledByDefault = input["enabledByDefault"] as Boolean as Boolean,
        toolCount = (input["toolCount"] as Number).toInt() as Int,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgSubpackageRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "packageName" to this.packageName,
    "containerPackageName" to this.containerPackageName,
    "subpackageId" to this.subpackageId,
    "entryPath" to this.entryPath,
    "displayName" to this.displayName.toMessagePackValue(),
    "description" to this.description.toMessagePackValue(),
    "enabledByDefault" to this.enabledByDefault,
    "toolCount" to this.toolCount,
)

data class ToolPkgTagFunctionHookRuntime(
    val id: String,
    val tag: String,
    val function: String,
    val functionSource: String?
)

fun decodeToolPkgTagFunctionHookRuntime(value: Any?): ToolPkgTagFunctionHookRuntime {
    val input = value as Map<*, *>
    return ToolPkgTagFunctionHookRuntime(
        id = input["id"] as String as String,
        tag = input["tag"] as String as String,
        function = input["function"] as String as String,
        functionSource = input["functionSource"]?.let { it as String } as String?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgTagFunctionHookRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "tag" to this.tag,
    "function" to this.function,
    "functionSource" to this.functionSource?.let { it },
)

data class ToolPkgUiModuleRuntime(
    val id: String,
    val runtime: String,
    val screen: String,
    val title: LocalizedText,
    val keepAlive: Boolean
)

fun decodeToolPkgUiModuleRuntime(value: Any?): ToolPkgUiModuleRuntime {
    val input = value as Map<*, *>
    return ToolPkgUiModuleRuntime(
        id = input["id"] as String as String,
        runtime = input["runtime"] as String as String,
        screen = input["screen"] as String as String,
        title = decodeLocalizedText(input["title"]) as LocalizedText,
        keepAlive = input["keepAlive"] as Boolean as Boolean,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgUiModuleRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "runtime" to this.runtime,
    "screen" to this.screen,
    "title" to this.title.toMessagePackValue(),
    "keepAlive" to this.keepAlive,
)

data class ToolPkgUiRouteRuntime(
    val id: String,
    val routeId: String,
    val runtime: String,
    val screen: String,
    val title: LocalizedText,
    val keepAlive: Boolean
)

fun decodeToolPkgUiRouteRuntime(value: Any?): ToolPkgUiRouteRuntime {
    val input = value as Map<*, *>
    return ToolPkgUiRouteRuntime(
        id = input["id"] as String as String,
        routeId = input["routeId"] as String as String,
        runtime = input["runtime"] as String as String,
        screen = input["screen"] as String as String,
        title = decodeLocalizedText(input["title"]) as LocalizedText,
        keepAlive = input["keepAlive"] as Boolean as Boolean,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgUiRouteRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "routeId" to this.routeId,
    "runtime" to this.runtime,
    "screen" to this.screen,
    "title" to this.title.toMessagePackValue(),
    "keepAlive" to this.keepAlive,
)

data class ToolPkgWasmModuleRuntime(
    val id: String,
    val path: String,
    val exports: List<String>,
    val sourceLanguage: String,
    val abi: String
)

fun decodeToolPkgWasmModuleRuntime(value: Any?): ToolPkgWasmModuleRuntime {
    val input = value as Map<*, *>
    return ToolPkgWasmModuleRuntime(
        id = input["id"] as String as String,
        path = input["path"] as String as String,
        exports = (input["exports"] as List<*>).map { item -> item as String } as List<String>,
        sourceLanguage = input["sourceLanguage"] as String as String,
        abi = input["abi"] as String as String,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgWasmModuleRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "path" to this.path,
    "exports" to this.exports.map { item -> item },
    "sourceLanguage" to this.sourceLanguage,
    "abi" to this.abi,
)

data class ToolPkgWorkflowTemplateRuntime(
    val id: String,
    val display_name: LocalizedText,
    val description: LocalizedText,
    val resource_key: String
)

fun decodeToolPkgWorkflowTemplateRuntime(value: Any?): ToolPkgWorkflowTemplateRuntime {
    val input = value as Map<*, *>
    return ToolPkgWorkflowTemplateRuntime(
        id = input["id"] as String as String,
        display_name = decodeLocalizedText(input["display_name"]) as LocalizedText,
        description = decodeLocalizedText(input["description"]) as LocalizedText,
        resource_key = input["resource_key"] as String as String,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgWorkflowTemplateRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "display_name" to this.display_name.toMessagePackValue(),
    "description" to this.description.toMessagePackValue(),
    "resource_key" to this.resource_key,
)

data class ToolPkgWorkspaceTemplateRuntime(
    val id: String,
    val display_name: LocalizedText,
    val description: LocalizedText,
    val resource_key: String,
    val project_type: String
)

fun decodeToolPkgWorkspaceTemplateRuntime(value: Any?): ToolPkgWorkspaceTemplateRuntime {
    val input = value as Map<*, *>
    return ToolPkgWorkspaceTemplateRuntime(
        id = input["id"] as String as String,
        display_name = decodeLocalizedText(input["display_name"]) as LocalizedText,
        description = decodeLocalizedText(input["description"]) as LocalizedText,
        resource_key = input["resource_key"] as String as String,
        project_type = input["project_type"] as String as String,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgWorkspaceTemplateRuntime.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "display_name" to this.display_name.toMessagePackValue(),
    "description" to this.description.toMessagePackValue(),
    "resource_key" to this.resource_key,
    "project_type" to this.project_type,
)

data class ToolResult(
    val toolName: String,
    val success: Boolean,
    val result: Any?,
    val error: String?
)

fun decodeToolResult(value: Any?): ToolResult {
    val input = value as Map<*, *>
    return ToolResult(
        toolName = input["toolName"] as String as String,
        success = input["success"] as Boolean as Boolean,
        result = input["result"] as Any?,
        error = input["error"]?.let { it as String } as String?,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolResult.toMessagePackValue(): Map<String, Any?> = mapOf(
    "toolName" to this.toolName,
    "success" to this.success,
    "result" to this.result,
    "error" to this.error?.let { it },
)

data class PluginLoadingItem(
    val id: String,
    val displayName: String,
    val kind: String,
    val status: String,
    val message: String,
    val logText: String
)

fun decodePluginLoadingItem(value: Any?): PluginLoadingItem {
    val input = value as Map<*, *>
    return PluginLoadingItem(
        id = input["id"] as String as String,
        displayName = input["displayName"] as String as String,
        kind = input["kind"] as String as String,
        status = input["status"] as String as String,
        message = input["message"] as String as String,
        logText = input["logText"] as String as String,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun PluginLoadingItem.toMessagePackValue(): Map<String, Any?> = mapOf(
    "id" to this.id,
    "displayName" to this.displayName,
    "kind" to this.kind,
    "status" to this.status,
    "message" to this.message,
    "logText" to this.logText,
)

data class PluginLoadingProgress(
    val visible: Boolean,
    val forceExpanded: Boolean,
    val progress: Double,
    val phase: String,
    val currentTask: String,
    val pluginsStarted: Int,
    val pluginsTotal: Int,
    val plugins: List<PluginLoadingItem>
)

fun decodePluginLoadingProgress(value: Any?): PluginLoadingProgress {
    val input = value as Map<*, *>
    return PluginLoadingProgress(
        visible = input["visible"] as Boolean as Boolean,
        forceExpanded = input["forceExpanded"] as Boolean as Boolean,
        progress = (input["progress"] as Number).toDouble() as Double,
        phase = input["phase"] as String as String,
        currentTask = input["currentTask"] as String as String,
        pluginsStarted = (input["pluginsStarted"] as Number).toInt() as Int,
        pluginsTotal = (input["pluginsTotal"] as Number).toInt() as Int,
        plugins = (input["plugins"] as List<*>).map { item -> decodePluginLoadingItem(item) } as List<PluginLoadingItem>,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun PluginLoadingProgress.toMessagePackValue(): Map<String, Any?> = mapOf(
    "visible" to this.visible,
    "forceExpanded" to this.forceExpanded,
    "progress" to this.progress,
    "phase" to this.phase,
    "currentTask" to this.currentTask,
    "pluginsStarted" to this.pluginsStarted,
    "pluginsTotal" to this.pluginsTotal,
    "plugins" to this.plugins.map { item -> item.toMessagePackValue() },
)

data class ToolPkgLoadIssue(
    val sourcePath: String,
    val packageName: String?,
    val displayName: String,
    val code: String,
    val message: String,
    val packageKind: String
)

fun decodeToolPkgLoadIssue(value: Any?): ToolPkgLoadIssue {
    val input = value as Map<*, *>
    return ToolPkgLoadIssue(
        sourcePath = input["sourcePath"] as String as String,
        packageName = input["packageName"]?.let { it as String } as String?,
        displayName = input["displayName"] as String as String,
        code = input["code"] as String as String,
        message = input["message"] as String as String,
        packageKind = input["packageKind"] as String as String,
    )
}

/** Encodes a typed SDK model into its Link argument representation. */
fun ToolPkgLoadIssue.toMessagePackValue(): Map<String, Any?> = mapOf(
    "sourcePath" to this.sourcePath,
    "packageName" to this.packageName?.let { it },
    "displayName" to this.displayName,
    "code" to this.code,
    "message" to this.message,
    "packageKind" to this.packageKind,
)

