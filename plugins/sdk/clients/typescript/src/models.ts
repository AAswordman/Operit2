// GENERATED FILE. Source: operit-proxy-scan.

export interface EnvVar {
  readonly name: string;
  readonly description: LocalizedText;
  readonly required: boolean;
  readonly default_value: string | null;
}

export function decodeEnvVar(value: unknown): EnvVar {
  const input = value as Record<string, unknown>;
  return {
    name: input['name'] as string,
    description: decodeLocalizedText(input['description']) as LocalizedText,
    required: input['required'] as boolean,
    default_value: input['default_value'] == null ? null : input['default_value'] as string | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeEnvVar(value: EnvVar): Record<string, unknown> {
  return {
    'name': value.name,
    'description': encodeLocalizedText(value.description),
    'required': value.required,
    'default_value': value.default_value === null ? null : value.default_value,
  };
}

export interface LocalizedText {
  readonly values: Record<string, string>;
}

export function decodeLocalizedText(value: unknown): LocalizedText {
  const input = value as Record<string, unknown>;
  return {
    values: input['values'] as Record<string, string>,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeLocalizedText(value: LocalizedText): Record<string, unknown> {
  return {
    'values': Object.fromEntries(Object.entries(value.values).map(([key, item]) => [key, item])),
  };
}

export interface PackageTool {
  readonly name: string;
  readonly description: LocalizedText;
  readonly parameters: Array<PackageToolParameter>;
  readonly script: string;
  readonly advice: boolean;
}

export function decodePackageTool(value: unknown): PackageTool {
  const input = value as Record<string, unknown>;
  return {
    name: input['name'] as string,
    description: decodeLocalizedText(input['description']) as LocalizedText,
    parameters: (input['parameters'] as unknown[]).map((item) => decodePackageToolParameter(item)) as Array<PackageToolParameter>,
    script: input['script'] as string,
    advice: input['advice'] as boolean,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodePackageTool(value: PackageTool): Record<string, unknown> {
  return {
    'name': value.name,
    'description': encodeLocalizedText(value.description),
    'parameters': value.parameters.map(item => encodePackageToolParameter(item)),
    'script': value.script,
    'advice': value.advice,
  };
}

export interface PackageToolParameter {
  readonly name: string;
  readonly description: LocalizedText;
  readonly parameter_type: string;
  readonly required: boolean;
}

export function decodePackageToolParameter(value: unknown): PackageToolParameter {
  const input = value as Record<string, unknown>;
  return {
    name: input['name'] as string,
    description: decodeLocalizedText(input['description']) as LocalizedText,
    parameter_type: input['parameter_type'] as string,
    required: input['required'] as boolean,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodePackageToolParameter(value: PackageToolParameter): Record<string, unknown> {
  return {
    'name': value.name,
    'description': encodeLocalizedText(value.description),
    'parameter_type': value.parameter_type,
    'required': value.required,
  };
}

export interface ToolPackage {
  readonly name: string;
  readonly description: LocalizedText;
  readonly tools: Array<PackageTool>;
  readonly states: Array<ToolPackageState>;
  readonly env: Array<EnvVar>;
  readonly is_built_in: boolean;
  readonly enabled_by_default: boolean;
  readonly display_name: LocalizedText;
  readonly category: string;
  readonly author: Array<string>;
}

export function decodeToolPackage(value: unknown): ToolPackage {
  const input = value as Record<string, unknown>;
  return {
    name: input['name'] as string,
    description: decodeLocalizedText(input['description']) as LocalizedText,
    tools: (input['tools'] as unknown[]).map((item) => decodePackageTool(item)) as Array<PackageTool>,
    states: (input['states'] as unknown[]).map((item) => decodeToolPackageState(item)) as Array<ToolPackageState>,
    env: (input['env'] as unknown[]).map((item) => decodeEnvVar(item)) as Array<EnvVar>,
    is_built_in: input['is_built_in'] as boolean,
    enabled_by_default: input['enabled_by_default'] as boolean,
    display_name: decodeLocalizedText(input['display_name']) as LocalizedText,
    category: input['category'] as string,
    author: (input['author'] as unknown[]).map((item) => item) as Array<string>,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPackage(value: ToolPackage): Record<string, unknown> {
  return {
    'name': value.name,
    'description': encodeLocalizedText(value.description),
    'tools': value.tools.map(item => encodePackageTool(item)),
    'states': value.states.map(item => encodeToolPackageState(item)),
    'env': value.env.map(item => encodeEnvVar(item)),
    'is_built_in': value.is_built_in,
    'enabled_by_default': value.enabled_by_default,
    'display_name': encodeLocalizedText(value.display_name),
    'category': value.category,
    'author': value.author.map(item => item),
  };
}

export interface ToolPackageState {
  readonly id: string;
  readonly condition: string;
  readonly inherit_tools: boolean;
  readonly exclude_tools: Array<string>;
  readonly tools: Array<PackageTool>;
}

export function decodeToolPackageState(value: unknown): ToolPackageState {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    condition: input['condition'] as string,
    inherit_tools: input['inherit_tools'] as boolean,
    exclude_tools: (input['exclude_tools'] as unknown[]).map((item) => item) as Array<string>,
    tools: (input['tools'] as unknown[]).map((item) => decodePackageTool(item)) as Array<PackageTool>,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPackageState(value: ToolPackageState): Record<string, unknown> {
  return {
    'id': value.id,
    'condition': value.condition,
    'inherit_tools': value.inherit_tools,
    'exclude_tools': value.exclude_tools.map(item => item),
    'tools': value.tools.map(item => encodePackageTool(item)),
  };
}

export interface ToolPkgContainerDetails {
  readonly packageName: string;
  readonly displayName: string;
  readonly description: string;
  readonly version: string;
  readonly apiVersion: string;
  readonly logoResourceKey: string | null;
  readonly logoMimeType: string | null;
  readonly author: Array<string>;
  readonly requires: Array<ToolPkgManifestRequirement>;
  readonly resourceCount: number;
  readonly workspaceTemplateCount: number;
  readonly uiModuleCount: number;
  readonly toolboxUiModules: Array<ToolPkgToolboxUiModule>;
  readonly subpackages: Array<ToolPkgSubpackageInfo>;
  readonly workspaceTemplates: Array<ToolPkgWorkspaceTemplate>;
}

export function decodeToolPkgContainerDetails(value: unknown): ToolPkgContainerDetails {
  const input = value as Record<string, unknown>;
  return {
    packageName: input['packageName'] as string,
    displayName: input['displayName'] as string,
    description: input['description'] as string,
    version: input['version'] as string,
    apiVersion: input['apiVersion'] as string,
    logoResourceKey: input['logoResourceKey'] == null ? null : input['logoResourceKey'] as string | null,
    logoMimeType: input['logoMimeType'] == null ? null : input['logoMimeType'] as string | null,
    author: (input['author'] as unknown[]).map((item) => item) as Array<string>,
    requires: (input['requires'] as unknown[]).map((item) => decodeToolPkgManifestRequirement(item)) as Array<ToolPkgManifestRequirement>,
    resourceCount: input['resourceCount'] as number,
    workspaceTemplateCount: input['workspaceTemplateCount'] as number,
    uiModuleCount: input['uiModuleCount'] as number,
    toolboxUiModules: (input['toolboxUiModules'] as unknown[]).map((item) => decodeToolPkgToolboxUiModule(item)) as Array<ToolPkgToolboxUiModule>,
    subpackages: (input['subpackages'] as unknown[]).map((item) => decodeToolPkgSubpackageInfo(item)) as Array<ToolPkgSubpackageInfo>,
    workspaceTemplates: (input['workspaceTemplates'] as unknown[]).map((item) => decodeToolPkgWorkspaceTemplate(item)) as Array<ToolPkgWorkspaceTemplate>,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgContainerDetails(value: ToolPkgContainerDetails): Record<string, unknown> {
  return {
    'packageName': value.packageName,
    'displayName': value.displayName,
    'description': value.description,
    'version': value.version,
    'apiVersion': value.apiVersion,
    'logoResourceKey': value.logoResourceKey === null ? null : value.logoResourceKey,
    'logoMimeType': value.logoMimeType === null ? null : value.logoMimeType,
    'author': value.author.map(item => item),
    'requires': value.requires.map(item => encodeToolPkgManifestRequirement(item)),
    'resourceCount': value.resourceCount,
    'workspaceTemplateCount': value.workspaceTemplateCount,
    'uiModuleCount': value.uiModuleCount,
    'toolboxUiModules': value.toolboxUiModules.map(item => encodeToolPkgToolboxUiModule(item)),
    'subpackages': value.subpackages.map(item => encodeToolPkgSubpackageInfo(item)),
    'workspaceTemplates': value.workspaceTemplates.map(item => encodeToolPkgWorkspaceTemplate(item)),
  };
}

export interface ToolPkgLogoBytes {
  readonly resourceKey: string;
  readonly mimeType: string;
  readonly fileName: string;
  readonly bytes: Uint8Array;
}

export function decodeToolPkgLogoBytes(value: unknown): ToolPkgLogoBytes {
  const input = value as Record<string, unknown>;
  return {
    resourceKey: input['resourceKey'] as string,
    mimeType: input['mimeType'] as string,
    fileName: input['fileName'] as string,
    bytes: new Uint8Array(input['bytes'] as number[]) as Uint8Array,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgLogoBytes(value: ToolPkgLogoBytes): Record<string, unknown> {
  return {
    'resourceKey': value.resourceKey,
    'mimeType': value.mimeType,
    'fileName': value.fileName,
    'bytes': Array.from(value.bytes),
  };
}

export interface ToolPkgSubpackageInfo {
  readonly packageName: string;
  readonly subpackageId: string;
  readonly displayName: string;
  readonly description: string;
  readonly enabledByDefault: boolean;
  readonly toolCount: number;
  readonly enabled: boolean;
}

export function decodeToolPkgSubpackageInfo(value: unknown): ToolPkgSubpackageInfo {
  const input = value as Record<string, unknown>;
  return {
    packageName: input['packageName'] as string,
    subpackageId: input['subpackageId'] as string,
    displayName: input['displayName'] as string,
    description: input['description'] as string,
    enabledByDefault: input['enabledByDefault'] as boolean,
    toolCount: input['toolCount'] as number,
    enabled: input['enabled'] as boolean,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgSubpackageInfo(value: ToolPkgSubpackageInfo): Record<string, unknown> {
  return {
    'packageName': value.packageName,
    'subpackageId': value.subpackageId,
    'displayName': value.displayName,
    'description': value.description,
    'enabledByDefault': value.enabledByDefault,
    'toolCount': value.toolCount,
    'enabled': value.enabled,
  };
}

export interface ToolPkgToolboxUiModule {
  readonly containerPackageName: string;
  readonly toolPkgId: string;
  readonly routeId: string;
  readonly uiModuleId: string;
  readonly runtime: string;
  readonly screen: string;
  readonly title: string;
  readonly description: string;
  readonly moduleSpec: Record<string, unknown>;
  readonly keepAlive: boolean;
}

export function decodeToolPkgToolboxUiModule(value: unknown): ToolPkgToolboxUiModule {
  const input = value as Record<string, unknown>;
  return {
    containerPackageName: input['containerPackageName'] as string,
    toolPkgId: input['toolPkgId'] as string,
    routeId: input['routeId'] as string,
    uiModuleId: input['uiModuleId'] as string,
    runtime: input['runtime'] as string,
    screen: input['screen'] as string,
    title: input['title'] as string,
    description: input['description'] as string,
    moduleSpec: input['moduleSpec'] as Record<string, unknown>,
    keepAlive: input['keepAlive'] as boolean,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgToolboxUiModule(value: ToolPkgToolboxUiModule): Record<string, unknown> {
  return {
    'containerPackageName': value.containerPackageName,
    'toolPkgId': value.toolPkgId,
    'routeId': value.routeId,
    'uiModuleId': value.uiModuleId,
    'runtime': value.runtime,
    'screen': value.screen,
    'title': value.title,
    'description': value.description,
    'moduleSpec': Object.fromEntries(Object.entries(value.moduleSpec).map(([key, item]) => [key, item])),
    'keepAlive': value.keepAlive,
  };
}

export interface ToolPkgWorkspaceTemplate {
  readonly containerPackageName: string;
  readonly toolPkgId: string;
  readonly templateId: string;
  readonly displayName: string;
  readonly description: string;
  readonly resourceKey: string;
  readonly projectType: string;
}

export function decodeToolPkgWorkspaceTemplate(value: unknown): ToolPkgWorkspaceTemplate {
  const input = value as Record<string, unknown>;
  return {
    containerPackageName: input['containerPackageName'] as string,
    toolPkgId: input['toolPkgId'] as string,
    templateId: input['templateId'] as string,
    displayName: input['displayName'] as string,
    description: input['description'] as string,
    resourceKey: input['resourceKey'] as string,
    projectType: input['projectType'] as string,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgWorkspaceTemplate(value: ToolPkgWorkspaceTemplate): Record<string, unknown> {
  return {
    'containerPackageName': value.containerPackageName,
    'toolPkgId': value.toolPkgId,
    'templateId': value.templateId,
    'displayName': value.displayName,
    'description': value.description,
    'resourceKey': value.resourceKey,
    'projectType': value.projectType,
  };
}

export interface ToolPkgAiProviderHandlerRuntime {
  readonly function: string;
  readonly functionSource: string | null;
}

export function decodeToolPkgAiProviderHandlerRuntime(value: unknown): ToolPkgAiProviderHandlerRuntime {
  const input = value as Record<string, unknown>;
  return {
    function: input['function'] as string,
    functionSource: input['functionSource'] == null ? null : input['functionSource'] as string | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgAiProviderHandlerRuntime(value: ToolPkgAiProviderHandlerRuntime): Record<string, unknown> {
  return {
    'function': value.function,
    'functionSource': value.functionSource === null ? null : value.functionSource,
  };
}

export interface ToolPkgAiProviderRuntime {
  readonly id: string;
  readonly displayName: string;
  readonly description: string;
  readonly listModelsHandler: ToolPkgAiProviderHandlerRuntime;
  readonly sendMessageHandler: ToolPkgAiProviderHandlerRuntime;
  readonly testConnectionHandler: ToolPkgAiProviderHandlerRuntime;
  readonly calculateInputTokensHandler: ToolPkgAiProviderHandlerRuntime;
}

export function decodeToolPkgAiProviderRuntime(value: unknown): ToolPkgAiProviderRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    displayName: input['displayName'] as string,
    description: input['description'] as string,
    listModelsHandler: decodeToolPkgAiProviderHandlerRuntime(input['listModelsHandler']) as ToolPkgAiProviderHandlerRuntime,
    sendMessageHandler: decodeToolPkgAiProviderHandlerRuntime(input['sendMessageHandler']) as ToolPkgAiProviderHandlerRuntime,
    testConnectionHandler: decodeToolPkgAiProviderHandlerRuntime(input['testConnectionHandler']) as ToolPkgAiProviderHandlerRuntime,
    calculateInputTokensHandler: decodeToolPkgAiProviderHandlerRuntime(input['calculateInputTokensHandler']) as ToolPkgAiProviderHandlerRuntime,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgAiProviderRuntime(value: ToolPkgAiProviderRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'displayName': value.displayName,
    'description': value.description,
    'listModelsHandler': encodeToolPkgAiProviderHandlerRuntime(value.listModelsHandler),
    'sendMessageHandler': encodeToolPkgAiProviderHandlerRuntime(value.sendMessageHandler),
    'testConnectionHandler': encodeToolPkgAiProviderHandlerRuntime(value.testConnectionHandler),
    'calculateInputTokensHandler': encodeToolPkgAiProviderHandlerRuntime(value.calculateInputTokensHandler),
  };
}

export interface ToolPkgAppLifecycleHookRuntime {
  readonly id: string;
  readonly event: string;
  readonly function: string;
  readonly functionSource: string | null;
}

export function decodeToolPkgAppLifecycleHookRuntime(value: unknown): ToolPkgAppLifecycleHookRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    event: input['event'] as string,
    function: input['function'] as string,
    functionSource: input['functionSource'] == null ? null : input['functionSource'] as string | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgAppLifecycleHookRuntime(value: ToolPkgAppLifecycleHookRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'event': value.event,
    'function': value.function,
    'functionSource': value.functionSource === null ? null : value.functionSource,
  };
}

export interface ToolPkgChatComposerSlotRuntime {
  readonly id: string;
  readonly slot: string;
  readonly screen: string;
  readonly order: number;
  readonly keepAlive: boolean;
}

export function decodeToolPkgChatComposerSlotRuntime(value: unknown): ToolPkgChatComposerSlotRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    slot: input['slot'] as string,
    screen: input['screen'] as string,
    order: input['order'] as number,
    keepAlive: input['keepAlive'] as boolean,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgChatComposerSlotRuntime(value: ToolPkgChatComposerSlotRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'slot': value.slot,
    'screen': value.screen,
    'order': value.order,
    'keepAlive': value.keepAlive,
  };
}

export interface ToolPkgChatMessageMenuDialogRuntime {
  readonly screen: string;
  readonly title: LocalizedText;
}

export function decodeToolPkgChatMessageMenuDialogRuntime(value: unknown): ToolPkgChatMessageMenuDialogRuntime {
  const input = value as Record<string, unknown>;
  return {
    screen: input['screen'] as string,
    title: decodeLocalizedText(input['title']) as LocalizedText,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgChatMessageMenuDialogRuntime(value: ToolPkgChatMessageMenuDialogRuntime): Record<string, unknown> {
  return {
    'screen': value.screen,
    'title': encodeLocalizedText(value.title),
  };
}

export interface ToolPkgChatMessageMenuItemRuntime {
  readonly id: string;
  readonly title: LocalizedText;
  readonly icon: string | null;
  readonly order: number;
  readonly senders: Array<string>;
  readonly function: string;
  readonly functionSource: string | null;
  readonly dialog: ToolPkgChatMessageMenuDialogRuntime | null;
}

export function decodeToolPkgChatMessageMenuItemRuntime(value: unknown): ToolPkgChatMessageMenuItemRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    title: decodeLocalizedText(input['title']) as LocalizedText,
    icon: input['icon'] == null ? null : input['icon'] as string | null,
    order: input['order'] as number,
    senders: (input['senders'] as unknown[]).map((item) => item) as Array<string>,
    function: input['function'] as string,
    functionSource: input['functionSource'] == null ? null : input['functionSource'] as string | null,
    dialog: input['dialog'] == null ? null : decodeToolPkgChatMessageMenuDialogRuntime(input['dialog']) as ToolPkgChatMessageMenuDialogRuntime | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgChatMessageMenuItemRuntime(value: ToolPkgChatMessageMenuItemRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'title': encodeLocalizedText(value.title),
    'icon': value.icon === null ? null : value.icon,
    'order': value.order,
    'senders': value.senders.map(item => item),
    'function': value.function,
    'functionSource': value.functionSource === null ? null : value.functionSource,
    'dialog': value.dialog === null ? null : encodeToolPkgChatMessageMenuDialogRuntime(value.dialog),
  };
}

export interface ToolPkgContainerRuntime {
  readonly packageName: string;
  readonly displayName: LocalizedText;
  readonly description: LocalizedText;
  readonly version: string;
  readonly apiVersion: string;
  readonly publicApi: string | null;
  readonly publicApis: Array<ToolPkgRegisteredFunctionHook>;
  readonly requires: Array<ToolPkgManifestRequirement>;
  readonly dependencyIssues: Array<ToolPkgDependencyIssue>;
  readonly manifestExtensions: Record<string, unknown>;
  readonly author: Array<string>;
  readonly mainEntry: string;
  readonly sourceType: ToolPkgSourceType;
  readonly sourcePath: string;
  readonly subpackages: Array<ToolPkgSubpackageRuntime>;
  readonly resources: Array<ToolPkgResourceRuntime>;
  readonly wasmModules: Array<ToolPkgWasmModuleRuntime>;
  readonly workflowTemplates: Array<ToolPkgWorkflowTemplateRuntime>;
  readonly workspaceTemplates: Array<ToolPkgWorkspaceTemplateRuntime>;
  readonly uiModules: Array<ToolPkgUiModuleRuntime>;
  readonly uiRoutes: Array<ToolPkgUiRouteRuntime>;
  readonly chatComposerSlots: Array<ToolPkgChatComposerSlotRuntime>;
  readonly navigationEntries: Array<ToolPkgNavigationEntryRuntime>;
  readonly desktopWidgets: Array<ToolPkgDesktopWidgetRuntime>;
  readonly appLifecycleHooks: Array<ToolPkgAppLifecycleHookRuntime>;
  readonly messageProcessingPlugins: Array<ToolPkgFunctionHookRuntime>;
  readonly xmlRenderPlugins: Array<ToolPkgTagFunctionHookRuntime>;
  readonly inputMenuTogglePlugins: Array<ToolPkgFunctionHookRuntime>;
  readonly chatInputHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly chatViewHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly chatMessageHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly chatLifecycleHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly chatMessageMenuItems: Array<ToolPkgChatMessageMenuItemRuntime>;
  readonly chatRuntimeHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly hostEventHooks: Array<ToolPkgHostEventHookRuntime>;
  readonly toolLifecycleHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly promptInputHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly promptHistoryHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly promptEstimateHistoryHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly systemPromptComposeHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly toolPromptComposeHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly promptFinalizeHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly promptEstimateFinalizeHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly summaryGenerateHooks: Array<ToolPkgFunctionHookRuntime>;
  readonly coreCommands: Array<ToolPkgCoreCommandRuntime>;
  readonly aiProviders: Array<ToolPkgAiProviderRuntime>;
  readonly manifestExtensionHandlers: Array<ToolPkgRegisteredManifestExtension>;
  readonly logoResource: ToolPkgResourceRuntime | null;
  readonly marketOrigin: ToolPkgMarketOrigin | null;
}

export function decodeToolPkgContainerRuntime(value: unknown): ToolPkgContainerRuntime {
  const input = value as Record<string, unknown>;
  return {
    packageName: input['packageName'] as string,
    displayName: decodeLocalizedText(input['displayName']) as LocalizedText,
    description: decodeLocalizedText(input['description']) as LocalizedText,
    version: input['version'] as string,
    apiVersion: input['apiVersion'] as string,
    publicApi: input['publicApi'] == null ? null : input['publicApi'] as string | null,
    publicApis: (input['publicApis'] as unknown[]).map((item) => decodeToolPkgRegisteredFunctionHook(item)) as Array<ToolPkgRegisteredFunctionHook>,
    requires: (input['requires'] as unknown[]).map((item) => decodeToolPkgManifestRequirement(item)) as Array<ToolPkgManifestRequirement>,
    dependencyIssues: (input['dependencyIssues'] as unknown[]).map((item) => decodeToolPkgDependencyIssue(item)) as Array<ToolPkgDependencyIssue>,
    manifestExtensions: input['manifestExtensions'] as Record<string, unknown>,
    author: (input['author'] as unknown[]).map((item) => item) as Array<string>,
    mainEntry: input['mainEntry'] as string,
    sourceType: decodeToolPkgSourceType(input['sourceType']) as ToolPkgSourceType,
    sourcePath: input['sourcePath'] as string,
    subpackages: (input['subpackages'] as unknown[]).map((item) => decodeToolPkgSubpackageRuntime(item)) as Array<ToolPkgSubpackageRuntime>,
    resources: (input['resources'] as unknown[]).map((item) => decodeToolPkgResourceRuntime(item)) as Array<ToolPkgResourceRuntime>,
    wasmModules: (input['wasmModules'] as unknown[]).map((item) => decodeToolPkgWasmModuleRuntime(item)) as Array<ToolPkgWasmModuleRuntime>,
    workflowTemplates: (input['workflowTemplates'] as unknown[]).map((item) => decodeToolPkgWorkflowTemplateRuntime(item)) as Array<ToolPkgWorkflowTemplateRuntime>,
    workspaceTemplates: (input['workspaceTemplates'] as unknown[]).map((item) => decodeToolPkgWorkspaceTemplateRuntime(item)) as Array<ToolPkgWorkspaceTemplateRuntime>,
    uiModules: (input['uiModules'] as unknown[]).map((item) => decodeToolPkgUiModuleRuntime(item)) as Array<ToolPkgUiModuleRuntime>,
    uiRoutes: (input['uiRoutes'] as unknown[]).map((item) => decodeToolPkgUiRouteRuntime(item)) as Array<ToolPkgUiRouteRuntime>,
    chatComposerSlots: (input['chatComposerSlots'] as unknown[]).map((item) => decodeToolPkgChatComposerSlotRuntime(item)) as Array<ToolPkgChatComposerSlotRuntime>,
    navigationEntries: (input['navigationEntries'] as unknown[]).map((item) => decodeToolPkgNavigationEntryRuntime(item)) as Array<ToolPkgNavigationEntryRuntime>,
    desktopWidgets: (input['desktopWidgets'] as unknown[]).map((item) => decodeToolPkgDesktopWidgetRuntime(item)) as Array<ToolPkgDesktopWidgetRuntime>,
    appLifecycleHooks: (input['appLifecycleHooks'] as unknown[]).map((item) => decodeToolPkgAppLifecycleHookRuntime(item)) as Array<ToolPkgAppLifecycleHookRuntime>,
    messageProcessingPlugins: (input['messageProcessingPlugins'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    xmlRenderPlugins: (input['xmlRenderPlugins'] as unknown[]).map((item) => decodeToolPkgTagFunctionHookRuntime(item)) as Array<ToolPkgTagFunctionHookRuntime>,
    inputMenuTogglePlugins: (input['inputMenuTogglePlugins'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    chatInputHooks: (input['chatInputHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    chatViewHooks: (input['chatViewHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    chatMessageHooks: (input['chatMessageHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    chatLifecycleHooks: (input['chatLifecycleHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    chatMessageMenuItems: (input['chatMessageMenuItems'] as unknown[]).map((item) => decodeToolPkgChatMessageMenuItemRuntime(item)) as Array<ToolPkgChatMessageMenuItemRuntime>,
    chatRuntimeHooks: (input['chatRuntimeHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    hostEventHooks: (input['hostEventHooks'] as unknown[]).map((item) => decodeToolPkgHostEventHookRuntime(item)) as Array<ToolPkgHostEventHookRuntime>,
    toolLifecycleHooks: (input['toolLifecycleHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    promptInputHooks: (input['promptInputHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    promptHistoryHooks: (input['promptHistoryHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    promptEstimateHistoryHooks: (input['promptEstimateHistoryHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    systemPromptComposeHooks: (input['systemPromptComposeHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    toolPromptComposeHooks: (input['toolPromptComposeHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    promptFinalizeHooks: (input['promptFinalizeHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    promptEstimateFinalizeHooks: (input['promptEstimateFinalizeHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    summaryGenerateHooks: (input['summaryGenerateHooks'] as unknown[]).map((item) => decodeToolPkgFunctionHookRuntime(item)) as Array<ToolPkgFunctionHookRuntime>,
    coreCommands: (input['coreCommands'] as unknown[]).map((item) => decodeToolPkgCoreCommandRuntime(item)) as Array<ToolPkgCoreCommandRuntime>,
    aiProviders: (input['aiProviders'] as unknown[]).map((item) => decodeToolPkgAiProviderRuntime(item)) as Array<ToolPkgAiProviderRuntime>,
    manifestExtensionHandlers: (input['manifestExtensionHandlers'] as unknown[]).map((item) => decodeToolPkgRegisteredManifestExtension(item)) as Array<ToolPkgRegisteredManifestExtension>,
    logoResource: input['logoResource'] == null ? null : decodeToolPkgResourceRuntime(input['logoResource']) as ToolPkgResourceRuntime | null,
    marketOrigin: input['marketOrigin'] == null ? null : decodeToolPkgMarketOrigin(input['marketOrigin']) as ToolPkgMarketOrigin | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgContainerRuntime(value: ToolPkgContainerRuntime): Record<string, unknown> {
  return {
    'packageName': value.packageName,
    'displayName': encodeLocalizedText(value.displayName),
    'description': encodeLocalizedText(value.description),
    'version': value.version,
    'apiVersion': value.apiVersion,
    'publicApi': value.publicApi === null ? null : value.publicApi,
    'publicApis': value.publicApis.map(item => encodeToolPkgRegisteredFunctionHook(item)),
    'requires': value.requires.map(item => encodeToolPkgManifestRequirement(item)),
    'dependencyIssues': value.dependencyIssues.map(item => encodeToolPkgDependencyIssue(item)),
    'manifestExtensions': Object.fromEntries(Object.entries(value.manifestExtensions).map(([key, item]) => [key, item])),
    'author': value.author.map(item => item),
    'mainEntry': value.mainEntry,
    'sourceType': encodeToolPkgSourceType(value.sourceType),
    'sourcePath': value.sourcePath,
    'subpackages': value.subpackages.map(item => encodeToolPkgSubpackageRuntime(item)),
    'resources': value.resources.map(item => encodeToolPkgResourceRuntime(item)),
    'wasmModules': value.wasmModules.map(item => encodeToolPkgWasmModuleRuntime(item)),
    'workflowTemplates': value.workflowTemplates.map(item => encodeToolPkgWorkflowTemplateRuntime(item)),
    'workspaceTemplates': value.workspaceTemplates.map(item => encodeToolPkgWorkspaceTemplateRuntime(item)),
    'uiModules': value.uiModules.map(item => encodeToolPkgUiModuleRuntime(item)),
    'uiRoutes': value.uiRoutes.map(item => encodeToolPkgUiRouteRuntime(item)),
    'chatComposerSlots': value.chatComposerSlots.map(item => encodeToolPkgChatComposerSlotRuntime(item)),
    'navigationEntries': value.navigationEntries.map(item => encodeToolPkgNavigationEntryRuntime(item)),
    'desktopWidgets': value.desktopWidgets.map(item => encodeToolPkgDesktopWidgetRuntime(item)),
    'appLifecycleHooks': value.appLifecycleHooks.map(item => encodeToolPkgAppLifecycleHookRuntime(item)),
    'messageProcessingPlugins': value.messageProcessingPlugins.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'xmlRenderPlugins': value.xmlRenderPlugins.map(item => encodeToolPkgTagFunctionHookRuntime(item)),
    'inputMenuTogglePlugins': value.inputMenuTogglePlugins.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'chatInputHooks': value.chatInputHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'chatViewHooks': value.chatViewHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'chatMessageHooks': value.chatMessageHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'chatLifecycleHooks': value.chatLifecycleHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'chatMessageMenuItems': value.chatMessageMenuItems.map(item => encodeToolPkgChatMessageMenuItemRuntime(item)),
    'chatRuntimeHooks': value.chatRuntimeHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'hostEventHooks': value.hostEventHooks.map(item => encodeToolPkgHostEventHookRuntime(item)),
    'toolLifecycleHooks': value.toolLifecycleHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'promptInputHooks': value.promptInputHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'promptHistoryHooks': value.promptHistoryHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'promptEstimateHistoryHooks': value.promptEstimateHistoryHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'systemPromptComposeHooks': value.systemPromptComposeHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'toolPromptComposeHooks': value.toolPromptComposeHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'promptFinalizeHooks': value.promptFinalizeHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'promptEstimateFinalizeHooks': value.promptEstimateFinalizeHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'summaryGenerateHooks': value.summaryGenerateHooks.map(item => encodeToolPkgFunctionHookRuntime(item)),
    'coreCommands': value.coreCommands.map(item => encodeToolPkgCoreCommandRuntime(item)),
    'aiProviders': value.aiProviders.map(item => encodeToolPkgAiProviderRuntime(item)),
    'manifestExtensionHandlers': value.manifestExtensionHandlers.map(item => encodeToolPkgRegisteredManifestExtension(item)),
    'logoResource': value.logoResource === null ? null : encodeToolPkgResourceRuntime(value.logoResource),
    'marketOrigin': value.marketOrigin === null ? null : encodeToolPkgMarketOrigin(value.marketOrigin),
  };
}

export interface ToolPkgCoreCommandRuntime {
  readonly id: string;
  readonly name: string;
  readonly title: LocalizedText;
  readonly description: LocalizedText;
  readonly usage: string;
  readonly function: string;
  readonly functionSource: string | null;
}

export function decodeToolPkgCoreCommandRuntime(value: unknown): ToolPkgCoreCommandRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    name: input['name'] as string,
    title: decodeLocalizedText(input['title']) as LocalizedText,
    description: decodeLocalizedText(input['description']) as LocalizedText,
    usage: input['usage'] as string,
    function: input['function'] as string,
    functionSource: input['functionSource'] == null ? null : input['functionSource'] as string | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgCoreCommandRuntime(value: ToolPkgCoreCommandRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'name': value.name,
    'title': encodeLocalizedText(value.title),
    'description': encodeLocalizedText(value.description),
    'usage': value.usage,
    'function': value.function,
    'functionSource': value.functionSource === null ? null : value.functionSource,
  };
}

export interface ToolPkgDependencyIssue {
  readonly id: string;
  readonly code: string;
  readonly requiredMinVersion: string | null;
  readonly requiredMaxVersion: string | null;
  readonly installedVersion: string | null;
  readonly enabled: boolean;
}

export function decodeToolPkgDependencyIssue(value: unknown): ToolPkgDependencyIssue {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    code: input['code'] as string,
    requiredMinVersion: input['requiredMinVersion'] == null ? null : input['requiredMinVersion'] as string | null,
    requiredMaxVersion: input['requiredMaxVersion'] == null ? null : input['requiredMaxVersion'] as string | null,
    installedVersion: input['installedVersion'] == null ? null : input['installedVersion'] as string | null,
    enabled: input['enabled'] as boolean,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgDependencyIssue(value: ToolPkgDependencyIssue): Record<string, unknown> {
  return {
    'id': value.id,
    'code': value.code,
    'requiredMinVersion': value.requiredMinVersion === null ? null : value.requiredMinVersion,
    'requiredMaxVersion': value.requiredMaxVersion === null ? null : value.requiredMaxVersion,
    'installedVersion': value.installedVersion === null ? null : value.installedVersion,
    'enabled': value.enabled,
  };
}

export interface ToolPkgDesktopWidgetRuntime {
  readonly id: string;
  readonly routeId: string;
  readonly renderRouteId: string;
  readonly title: LocalizedText;
  readonly subtitle: LocalizedText;
  readonly description: LocalizedText;
  readonly icon: string | null;
  readonly order: number;
}

export function decodeToolPkgDesktopWidgetRuntime(value: unknown): ToolPkgDesktopWidgetRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    routeId: input['routeId'] as string,
    renderRouteId: input['renderRouteId'] as string,
    title: decodeLocalizedText(input['title']) as LocalizedText,
    subtitle: decodeLocalizedText(input['subtitle']) as LocalizedText,
    description: decodeLocalizedText(input['description']) as LocalizedText,
    icon: input['icon'] == null ? null : input['icon'] as string | null,
    order: input['order'] as number,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgDesktopWidgetRuntime(value: ToolPkgDesktopWidgetRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'routeId': value.routeId,
    'renderRouteId': value.renderRouteId,
    'title': encodeLocalizedText(value.title),
    'subtitle': encodeLocalizedText(value.subtitle),
    'description': encodeLocalizedText(value.description),
    'icon': value.icon === null ? null : value.icon,
    'order': value.order,
  };
}

export interface ToolPkgFunctionHookRuntime {
  readonly id: string;
  readonly function: string;
  readonly functionSource: string | null;
}

export function decodeToolPkgFunctionHookRuntime(value: unknown): ToolPkgFunctionHookRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    function: input['function'] as string,
    functionSource: input['functionSource'] == null ? null : input['functionSource'] as string | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgFunctionHookRuntime(value: ToolPkgFunctionHookRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'function': value.function,
    'functionSource': value.functionSource === null ? null : value.functionSource,
  };
}

export interface ToolPkgHostEventHookRuntime {
  readonly id: string;
  readonly source: string;
  readonly trigger: unknown;
  readonly function: string;
  readonly functionSource: string | null;
  readonly enabled: boolean;
}

export function decodeToolPkgHostEventHookRuntime(value: unknown): ToolPkgHostEventHookRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    source: input['source'] as string,
    trigger: input['trigger'] as unknown,
    function: input['function'] as string,
    functionSource: input['functionSource'] == null ? null : input['functionSource'] as string | null,
    enabled: input['enabled'] as boolean,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgHostEventHookRuntime(value: ToolPkgHostEventHookRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'source': value.source,
    'trigger': value.trigger,
    'function': value.function,
    'functionSource': value.functionSource === null ? null : value.functionSource,
    'enabled': value.enabled,
  };
}

export interface ToolPkgManifestRequirement {
  readonly id: string;
  readonly description: string;
  readonly minVersion: string | null;
  readonly maxVersion: string | null;
}

export function decodeToolPkgManifestRequirement(value: unknown): ToolPkgManifestRequirement {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    description: input['description'] as string,
    minVersion: input['min_version'] == null ? null : input['min_version'] as string | null,
    maxVersion: input['max_version'] == null ? null : input['max_version'] as string | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgManifestRequirement(value: ToolPkgManifestRequirement): Record<string, unknown> {
  return {
    'id': value.id,
    'description': value.description,
    'min_version': value.minVersion === null ? null : value.minVersion,
    'max_version': value.maxVersion === null ? null : value.maxVersion,
  };
}

export interface ToolPkgMarketOrigin {
  readonly market: string;
  readonly toolpkgId: string;
  readonly version: string;
  readonly author: Array<string>;
}

export function decodeToolPkgMarketOrigin(value: unknown): ToolPkgMarketOrigin {
  const input = value as Record<string, unknown>;
  return {
    market: input['market'] as string,
    toolpkgId: input['toolpkgId'] as string,
    version: input['version'] as string,
    author: (input['author'] as unknown[]).map((item) => item) as Array<string>,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgMarketOrigin(value: ToolPkgMarketOrigin): Record<string, unknown> {
  return {
    'market': value.market,
    'toolpkgId': value.toolpkgId,
    'version': value.version,
    'author': value.author.map(item => item),
  };
}

export interface ToolPkgNavigationActionHookRuntime {
  readonly function: string;
  readonly functionSource: string | null;
}

export function decodeToolPkgNavigationActionHookRuntime(value: unknown): ToolPkgNavigationActionHookRuntime {
  const input = value as Record<string, unknown>;
  return {
    function: input['function'] as string,
    functionSource: input['functionSource'] == null ? null : input['functionSource'] as string | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgNavigationActionHookRuntime(value: ToolPkgNavigationActionHookRuntime): Record<string, unknown> {
  return {
    'function': value.function,
    'functionSource': value.functionSource === null ? null : value.functionSource,
  };
}

export interface ToolPkgNavigationEntryRuntime {
  readonly id: string;
  readonly routeId: string;
  readonly params: unknown;
  readonly surface: string;
  readonly title: LocalizedText;
  readonly action: ToolPkgNavigationActionHookRuntime | null;
  readonly icon: string | null;
  readonly order: number;
}

export function decodeToolPkgNavigationEntryRuntime(value: unknown): ToolPkgNavigationEntryRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    routeId: input['routeId'] as string,
    params: input['params'] as unknown,
    surface: input['surface'] as string,
    title: decodeLocalizedText(input['title']) as LocalizedText,
    action: input['action'] == null ? null : decodeToolPkgNavigationActionHookRuntime(input['action']) as ToolPkgNavigationActionHookRuntime | null,
    icon: input['icon'] == null ? null : input['icon'] as string | null,
    order: input['order'] as number,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgNavigationEntryRuntime(value: ToolPkgNavigationEntryRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'routeId': value.routeId,
    'params': value.params,
    'surface': value.surface,
    'title': encodeLocalizedText(value.title),
    'action': value.action === null ? null : encodeToolPkgNavigationActionHookRuntime(value.action),
    'icon': value.icon === null ? null : value.icon,
    'order': value.order,
  };
}

export interface ToolPkgRegisteredFunctionHook {
  readonly id: string;
  readonly function: string;
  readonly functionSource: string | null;
}

export function decodeToolPkgRegisteredFunctionHook(value: unknown): ToolPkgRegisteredFunctionHook {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    function: input['function'] as string,
    functionSource: input['functionSource'] == null ? null : input['functionSource'] as string | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgRegisteredFunctionHook(value: ToolPkgRegisteredFunctionHook): Record<string, unknown> {
  return {
    'id': value.id,
    'function': value.function,
    'functionSource': value.functionSource === null ? null : value.functionSource,
  };
}

export interface ToolPkgRegisteredManifestExtension {
  readonly key: string;
  readonly function: string;
  readonly functionSource: string | null;
}

export function decodeToolPkgRegisteredManifestExtension(value: unknown): ToolPkgRegisteredManifestExtension {
  const input = value as Record<string, unknown>;
  return {
    key: input['key'] as string,
    function: input['function'] as string,
    functionSource: input['functionSource'] == null ? null : input['functionSource'] as string | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgRegisteredManifestExtension(value: ToolPkgRegisteredManifestExtension): Record<string, unknown> {
  return {
    'key': value.key,
    'function': value.function,
    'functionSource': value.functionSource === null ? null : value.functionSource,
  };
}

export interface ToolPkgResourceRuntime {
  readonly key: string;
  readonly path: string;
  readonly mime: string;
}

export function decodeToolPkgResourceRuntime(value: unknown): ToolPkgResourceRuntime {
  const input = value as Record<string, unknown>;
  return {
    key: input['key'] as string,
    path: input['path'] as string,
    mime: input['mime'] as string,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgResourceRuntime(value: ToolPkgResourceRuntime): Record<string, unknown> {
  return {
    'key': value.key,
    'path': value.path,
    'mime': value.mime,
  };
}

export type ToolPkgSourceType = 'ASSET' | 'MARKET' | 'EXTERNAL';
export function decodeToolPkgSourceType(value: unknown): ToolPkgSourceType { return value as ToolPkgSourceType; }

/** Encodes the declared Link enum scalar. */
export function encodeToolPkgSourceType(value: ToolPkgSourceType): string { return value; }

export interface ToolPkgSubpackageRuntime {
  readonly packageName: string;
  readonly containerPackageName: string;
  readonly subpackageId: string;
  readonly entryPath: string;
  readonly displayName: LocalizedText;
  readonly description: LocalizedText;
  readonly enabledByDefault: boolean;
  readonly toolCount: number;
}

export function decodeToolPkgSubpackageRuntime(value: unknown): ToolPkgSubpackageRuntime {
  const input = value as Record<string, unknown>;
  return {
    packageName: input['packageName'] as string,
    containerPackageName: input['containerPackageName'] as string,
    subpackageId: input['subpackageId'] as string,
    entryPath: input['entryPath'] as string,
    displayName: decodeLocalizedText(input['displayName']) as LocalizedText,
    description: decodeLocalizedText(input['description']) as LocalizedText,
    enabledByDefault: input['enabledByDefault'] as boolean,
    toolCount: input['toolCount'] as number,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgSubpackageRuntime(value: ToolPkgSubpackageRuntime): Record<string, unknown> {
  return {
    'packageName': value.packageName,
    'containerPackageName': value.containerPackageName,
    'subpackageId': value.subpackageId,
    'entryPath': value.entryPath,
    'displayName': encodeLocalizedText(value.displayName),
    'description': encodeLocalizedText(value.description),
    'enabledByDefault': value.enabledByDefault,
    'toolCount': value.toolCount,
  };
}

export interface ToolPkgTagFunctionHookRuntime {
  readonly id: string;
  readonly tag: string;
  readonly function: string;
  readonly functionSource: string | null;
}

export function decodeToolPkgTagFunctionHookRuntime(value: unknown): ToolPkgTagFunctionHookRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    tag: input['tag'] as string,
    function: input['function'] as string,
    functionSource: input['functionSource'] == null ? null : input['functionSource'] as string | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgTagFunctionHookRuntime(value: ToolPkgTagFunctionHookRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'tag': value.tag,
    'function': value.function,
    'functionSource': value.functionSource === null ? null : value.functionSource,
  };
}

export interface ToolPkgUiModuleRuntime {
  readonly id: string;
  readonly runtime: string;
  readonly screen: string;
  readonly title: LocalizedText;
  readonly keepAlive: boolean;
}

export function decodeToolPkgUiModuleRuntime(value: unknown): ToolPkgUiModuleRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    runtime: input['runtime'] as string,
    screen: input['screen'] as string,
    title: decodeLocalizedText(input['title']) as LocalizedText,
    keepAlive: input['keepAlive'] as boolean,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgUiModuleRuntime(value: ToolPkgUiModuleRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'runtime': value.runtime,
    'screen': value.screen,
    'title': encodeLocalizedText(value.title),
    'keepAlive': value.keepAlive,
  };
}

export interface ToolPkgUiRouteRuntime {
  readonly id: string;
  readonly routeId: string;
  readonly runtime: string;
  readonly screen: string;
  readonly title: LocalizedText;
  readonly keepAlive: boolean;
}

export function decodeToolPkgUiRouteRuntime(value: unknown): ToolPkgUiRouteRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    routeId: input['routeId'] as string,
    runtime: input['runtime'] as string,
    screen: input['screen'] as string,
    title: decodeLocalizedText(input['title']) as LocalizedText,
    keepAlive: input['keepAlive'] as boolean,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgUiRouteRuntime(value: ToolPkgUiRouteRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'routeId': value.routeId,
    'runtime': value.runtime,
    'screen': value.screen,
    'title': encodeLocalizedText(value.title),
    'keepAlive': value.keepAlive,
  };
}

export interface ToolPkgWasmModuleRuntime {
  readonly id: string;
  readonly path: string;
  readonly exports: Array<string>;
  readonly sourceLanguage: string;
  readonly abi: string;
}

export function decodeToolPkgWasmModuleRuntime(value: unknown): ToolPkgWasmModuleRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    path: input['path'] as string,
    exports: (input['exports'] as unknown[]).map((item) => item) as Array<string>,
    sourceLanguage: input['sourceLanguage'] as string,
    abi: input['abi'] as string,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgWasmModuleRuntime(value: ToolPkgWasmModuleRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'path': value.path,
    'exports': value.exports.map(item => item),
    'sourceLanguage': value.sourceLanguage,
    'abi': value.abi,
  };
}

export interface ToolPkgWorkflowTemplateRuntime {
  readonly id: string;
  readonly display_name: LocalizedText;
  readonly description: LocalizedText;
  readonly resource_key: string;
}

export function decodeToolPkgWorkflowTemplateRuntime(value: unknown): ToolPkgWorkflowTemplateRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    display_name: decodeLocalizedText(input['display_name']) as LocalizedText,
    description: decodeLocalizedText(input['description']) as LocalizedText,
    resource_key: input['resource_key'] as string,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgWorkflowTemplateRuntime(value: ToolPkgWorkflowTemplateRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'display_name': encodeLocalizedText(value.display_name),
    'description': encodeLocalizedText(value.description),
    'resource_key': value.resource_key,
  };
}

export interface ToolPkgWorkspaceTemplateRuntime {
  readonly id: string;
  readonly display_name: LocalizedText;
  readonly description: LocalizedText;
  readonly resource_key: string;
  readonly project_type: string;
}

export function decodeToolPkgWorkspaceTemplateRuntime(value: unknown): ToolPkgWorkspaceTemplateRuntime {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    display_name: decodeLocalizedText(input['display_name']) as LocalizedText,
    description: decodeLocalizedText(input['description']) as LocalizedText,
    resource_key: input['resource_key'] as string,
    project_type: input['project_type'] as string,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgWorkspaceTemplateRuntime(value: ToolPkgWorkspaceTemplateRuntime): Record<string, unknown> {
  return {
    'id': value.id,
    'display_name': encodeLocalizedText(value.display_name),
    'description': encodeLocalizedText(value.description),
    'resource_key': value.resource_key,
    'project_type': value.project_type,
  };
}

export interface ToolResult {
  readonly toolName: string;
  readonly success: boolean;
  readonly result: unknown;
  readonly error: string | null;
}

export function decodeToolResult(value: unknown): ToolResult {
  const input = value as Record<string, unknown>;
  return {
    toolName: input['toolName'] as string,
    success: input['success'] as boolean,
    result: input['result'] as unknown,
    error: input['error'] == null ? null : input['error'] as string | null,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolResult(value: ToolResult): Record<string, unknown> {
  return {
    'toolName': value.toolName,
    'success': value.success,
    'result': value.result,
    'error': value.error === null ? null : value.error,
  };
}

export interface PluginLoadingItem {
  readonly id: string;
  readonly displayName: string;
  readonly kind: string;
  readonly status: string;
  readonly message: string;
  readonly logText: string;
}

export function decodePluginLoadingItem(value: unknown): PluginLoadingItem {
  const input = value as Record<string, unknown>;
  return {
    id: input['id'] as string,
    displayName: input['displayName'] as string,
    kind: input['kind'] as string,
    status: input['status'] as string,
    message: input['message'] as string,
    logText: input['logText'] as string,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodePluginLoadingItem(value: PluginLoadingItem): Record<string, unknown> {
  return {
    'id': value.id,
    'displayName': value.displayName,
    'kind': value.kind,
    'status': value.status,
    'message': value.message,
    'logText': value.logText,
  };
}

export interface PluginLoadingProgress {
  readonly visible: boolean;
  readonly forceExpanded: boolean;
  readonly progress: number;
  readonly phase: string;
  readonly currentTask: string;
  readonly pluginsStarted: number;
  readonly pluginsTotal: number;
  readonly plugins: Array<PluginLoadingItem>;
}

export function decodePluginLoadingProgress(value: unknown): PluginLoadingProgress {
  const input = value as Record<string, unknown>;
  return {
    visible: input['visible'] as boolean,
    forceExpanded: input['forceExpanded'] as boolean,
    progress: input['progress'] as number,
    phase: input['phase'] as string,
    currentTask: input['currentTask'] as string,
    pluginsStarted: input['pluginsStarted'] as number,
    pluginsTotal: input['pluginsTotal'] as number,
    plugins: (input['plugins'] as unknown[]).map((item) => decodePluginLoadingItem(item)) as Array<PluginLoadingItem>,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodePluginLoadingProgress(value: PluginLoadingProgress): Record<string, unknown> {
  return {
    'visible': value.visible,
    'forceExpanded': value.forceExpanded,
    'progress': value.progress,
    'phase': value.phase,
    'currentTask': value.currentTask,
    'pluginsStarted': value.pluginsStarted,
    'pluginsTotal': value.pluginsTotal,
    'plugins': value.plugins.map(item => encodePluginLoadingItem(item)),
  };
}

export interface ToolPkgLoadIssue {
  readonly sourcePath: string;
  readonly packageName: string | null;
  readonly displayName: string;
  readonly code: string;
  readonly message: string;
  readonly packageKind: string;
}

export function decodeToolPkgLoadIssue(value: unknown): ToolPkgLoadIssue {
  const input = value as Record<string, unknown>;
  return {
    sourcePath: input['sourcePath'] as string,
    packageName: input['packageName'] == null ? null : input['packageName'] as string | null,
    displayName: input['displayName'] as string,
    code: input['code'] as string,
    message: input['message'] as string,
    packageKind: input['packageKind'] as string,
  };
}

/** Encodes a typed SDK model into its Link argument representation. */
export function encodeToolPkgLoadIssue(value: ToolPkgLoadIssue): Record<string, unknown> {
  return {
    'sourcePath': value.sourcePath,
    'packageName': value.packageName === null ? null : value.packageName,
    'displayName': value.displayName,
    'code': value.code,
    'message': value.message,
    'packageKind': value.packageKind,
  };
}

