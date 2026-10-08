// GENERATED FILE. Source: operit-proxy-scan.

import 'dart:typed_data';

/// Generated SDK model for Rust type `operit_plugin_sdk::package::EnvVar`.
class EnvVar {
  const EnvVar({
    required this.name,
    required this.description,
    required this.required,
    required this.default_value,
  });

  /// Decodes `operit_plugin_sdk::package::EnvVar` from a MessagePack value map.
  factory EnvVar.fromMessagePackValue(Map<String, Object?> value) => EnvVar(
    name: value['name'] as String,
    description: LocalizedText.fromMessagePackValue(value['description'] as Map<String, Object?>),
    required: value['required'] as bool,
    default_value: value['default_value'] == null ? null : value['default_value'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'name': name,
    'description': description.toMessagePackValue(),
    'required': required,
    'default_value': default_value == null ? null : default_value!,
  };

  final String name;
  final LocalizedText description;
  final bool required;
  final String? default_value;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::package::LocalizedText`.
class LocalizedText {
  const LocalizedText({
    required this.values,
  });

  /// Decodes `operit_plugin_sdk::package::LocalizedText` from a MessagePack value map.
  factory LocalizedText.fromMessagePackValue(Map<String, Object?> value) => LocalizedText(
    values: (value['values'] as Map).map((key, item) => MapEntry(key as String, item as String)),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'values': values.map((key, item) => MapEntry(key, item)),
  };

  final Map<String, String> values;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::package::PackageTool`.
class PackageTool {
  const PackageTool({
    required this.name,
    required this.description,
    required this.parameters,
    required this.script,
    required this.advice,
  });

  /// Decodes `operit_plugin_sdk::package::PackageTool` from a MessagePack value map.
  factory PackageTool.fromMessagePackValue(Map<String, Object?> value) => PackageTool(
    name: value['name'] as String,
    description: LocalizedText.fromMessagePackValue(value['description'] as Map<String, Object?>),
    parameters: (value['parameters'] as List<Object?>).map((item) => PackageToolParameter.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    script: value['script'] as String,
    advice: value['advice'] as bool,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'name': name,
    'description': description.toMessagePackValue(),
    'parameters': parameters.map((item) => item.toMessagePackValue()).toList(growable: false),
    'script': script,
    'advice': advice,
  };

  final String name;
  final LocalizedText description;
  final List<PackageToolParameter> parameters;
  final String script;
  final bool advice;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::package::PackageToolParameter`.
class PackageToolParameter {
  const PackageToolParameter({
    required this.name,
    required this.description,
    required this.parameter_type,
    required this.required,
  });

  /// Decodes `operit_plugin_sdk::package::PackageToolParameter` from a MessagePack value map.
  factory PackageToolParameter.fromMessagePackValue(Map<String, Object?> value) => PackageToolParameter(
    name: value['name'] as String,
    description: LocalizedText.fromMessagePackValue(value['description'] as Map<String, Object?>),
    parameter_type: value['parameter_type'] as String,
    required: value['required'] as bool,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'name': name,
    'description': description.toMessagePackValue(),
    'parameter_type': parameter_type,
    'required': required,
  };

  final String name;
  final LocalizedText description;
  final String parameter_type;
  final bool required;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::package::ToolPackage`.
class ToolPackage {
  const ToolPackage({
    required this.name,
    required this.description,
    required this.tools,
    required this.states,
    required this.env,
    required this.is_built_in,
    required this.enabled_by_default,
    required this.display_name,
    required this.category,
    required this.author,
  });

  /// Decodes `operit_plugin_sdk::package::ToolPackage` from a MessagePack value map.
  factory ToolPackage.fromMessagePackValue(Map<String, Object?> value) => ToolPackage(
    name: value['name'] as String,
    description: LocalizedText.fromMessagePackValue(value['description'] as Map<String, Object?>),
    tools: (value['tools'] as List<Object?>).map((item) => PackageTool.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    states: (value['states'] as List<Object?>).map((item) => ToolPackageState.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    env: (value['env'] as List<Object?>).map((item) => EnvVar.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    is_built_in: value['is_built_in'] as bool,
    enabled_by_default: value['enabled_by_default'] as bool,
    display_name: LocalizedText.fromMessagePackValue(value['display_name'] as Map<String, Object?>),
    category: value['category'] as String,
    author: (value['author'] as List<Object?>).map((item) => item as String).toList(growable: false),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'name': name,
    'description': description.toMessagePackValue(),
    'tools': tools.map((item) => item.toMessagePackValue()).toList(growable: false),
    'states': states.map((item) => item.toMessagePackValue()).toList(growable: false),
    'env': env.map((item) => item.toMessagePackValue()).toList(growable: false),
    'is_built_in': is_built_in,
    'enabled_by_default': enabled_by_default,
    'display_name': display_name.toMessagePackValue(),
    'category': category,
    'author': author.map((item) => item).toList(growable: false),
  };

  final String name;
  final LocalizedText description;
  final List<PackageTool> tools;
  final List<ToolPackageState> states;
  final List<EnvVar> env;
  final bool is_built_in;
  final bool enabled_by_default;
  final LocalizedText display_name;
  final String category;
  final List<String> author;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::package::ToolPackageState`.
class ToolPackageState {
  const ToolPackageState({
    required this.id,
    required this.condition,
    required this.inherit_tools,
    required this.exclude_tools,
    required this.tools,
  });

  /// Decodes `operit_plugin_sdk::package::ToolPackageState` from a MessagePack value map.
  factory ToolPackageState.fromMessagePackValue(Map<String, Object?> value) => ToolPackageState(
    id: value['id'] as String,
    condition: value['condition'] as String,
    inherit_tools: value['inherit_tools'] as bool,
    exclude_tools: (value['exclude_tools'] as List<Object?>).map((item) => item as String).toList(growable: false),
    tools: (value['tools'] as List<Object?>).map((item) => PackageTool.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'condition': condition,
    'inherit_tools': inherit_tools,
    'exclude_tools': exclude_tools.map((item) => item).toList(growable: false),
    'tools': tools.map((item) => item.toMessagePackValue()).toList(growable: false),
  };

  final String id;
  final String condition;
  final bool inherit_tools;
  final List<String> exclude_tools;
  final List<PackageTool> tools;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgContainerDetails`.
class ToolPkgContainerDetails {
  const ToolPkgContainerDetails({
    required this.packageName,
    required this.displayName,
    required this.description,
    required this.version,
    required this.apiVersion,
    required this.logoResourceKey,
    required this.logoMimeType,
    required this.author,
    required this.requires,
    required this.resourceCount,
    required this.workspaceTemplateCount,
    required this.uiModuleCount,
    required this.toolboxUiModules,
    required this.subpackages,
    required this.workspaceTemplates,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgContainerDetails` from a MessagePack value map.
  factory ToolPkgContainerDetails.fromMessagePackValue(Map<String, Object?> value) => ToolPkgContainerDetails(
    packageName: value['packageName'] as String,
    displayName: value['displayName'] as String,
    description: value['description'] as String,
    version: value['version'] as String,
    apiVersion: value['apiVersion'] as String,
    logoResourceKey: value['logoResourceKey'] == null ? null : value['logoResourceKey'] as String,
    logoMimeType: value['logoMimeType'] == null ? null : value['logoMimeType'] as String,
    author: (value['author'] as List<Object?>).map((item) => item as String).toList(growable: false),
    requires: (value['requires'] as List<Object?>).map((item) => ToolPkgManifestRequirement.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    resourceCount: (value['resourceCount'] as num).toInt(),
    workspaceTemplateCount: (value['workspaceTemplateCount'] as num).toInt(),
    uiModuleCount: (value['uiModuleCount'] as num).toInt(),
    toolboxUiModules: (value['toolboxUiModules'] as List<Object?>).map((item) => ToolPkgToolboxUiModule.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    subpackages: (value['subpackages'] as List<Object?>).map((item) => ToolPkgSubpackageInfo.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    workspaceTemplates: (value['workspaceTemplates'] as List<Object?>).map((item) => ToolPkgWorkspaceTemplate.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'packageName': packageName,
    'displayName': displayName,
    'description': description,
    'version': version,
    'apiVersion': apiVersion,
    'logoResourceKey': logoResourceKey == null ? null : logoResourceKey!,
    'logoMimeType': logoMimeType == null ? null : logoMimeType!,
    'author': author.map((item) => item).toList(growable: false),
    'requires': requires.map((item) => item.toMessagePackValue()).toList(growable: false),
    'resourceCount': resourceCount,
    'workspaceTemplateCount': workspaceTemplateCount,
    'uiModuleCount': uiModuleCount,
    'toolboxUiModules': toolboxUiModules.map((item) => item.toMessagePackValue()).toList(growable: false),
    'subpackages': subpackages.map((item) => item.toMessagePackValue()).toList(growable: false),
    'workspaceTemplates': workspaceTemplates.map((item) => item.toMessagePackValue()).toList(growable: false),
  };

  final String packageName;
  final String displayName;
  final String description;
  final String version;
  final String apiVersion;
  final String? logoResourceKey;
  final String? logoMimeType;
  final List<String> author;
  final List<ToolPkgManifestRequirement> requires;
  final int resourceCount;
  final int workspaceTemplateCount;
  final int uiModuleCount;
  final List<ToolPkgToolboxUiModule> toolboxUiModules;
  final List<ToolPkgSubpackageInfo> subpackages;
  final List<ToolPkgWorkspaceTemplate> workspaceTemplates;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgLogoBytes`.
class ToolPkgLogoBytes {
  const ToolPkgLogoBytes({
    required this.resourceKey,
    required this.mimeType,
    required this.fileName,
    required this.bytes,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgLogoBytes` from a MessagePack value map.
  factory ToolPkgLogoBytes.fromMessagePackValue(Map<String, Object?> value) => ToolPkgLogoBytes(
    resourceKey: value['resourceKey'] as String,
    mimeType: value['mimeType'] as String,
    fileName: value['fileName'] as String,
    bytes: Uint8List.fromList(((value['bytes'] as List).cast<int>())),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'resourceKey': resourceKey,
    'mimeType': mimeType,
    'fileName': fileName,
    'bytes': bytes.toList(growable: false),
  };

  final String resourceKey;
  final String mimeType;
  final String fileName;
  final Uint8List bytes;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgSubpackageInfo`.
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

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgSubpackageInfo` from a MessagePack value map.
  factory ToolPkgSubpackageInfo.fromMessagePackValue(Map<String, Object?> value) => ToolPkgSubpackageInfo(
    packageName: value['packageName'] as String,
    subpackageId: value['subpackageId'] as String,
    displayName: value['displayName'] as String,
    description: value['description'] as String,
    enabledByDefault: value['enabledByDefault'] as bool,
    toolCount: (value['toolCount'] as num).toInt(),
    enabled: value['enabled'] as bool,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'packageName': packageName,
    'subpackageId': subpackageId,
    'displayName': displayName,
    'description': description,
    'enabledByDefault': enabledByDefault,
    'toolCount': toolCount,
    'enabled': enabled,
  };

  final String packageName;
  final String subpackageId;
  final String displayName;
  final String description;
  final bool enabledByDefault;
  final int toolCount;
  final bool enabled;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgToolboxUiModule`.
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

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgToolboxUiModule` from a MessagePack value map.
  factory ToolPkgToolboxUiModule.fromMessagePackValue(Map<String, Object?> value) => ToolPkgToolboxUiModule(
    containerPackageName: value['containerPackageName'] as String,
    toolPkgId: value['toolPkgId'] as String,
    routeId: value['routeId'] as String,
    uiModuleId: value['uiModuleId'] as String,
    runtime: value['runtime'] as String,
    screen: value['screen'] as String,
    title: value['title'] as String,
    description: value['description'] as String,
    moduleSpec: (value['moduleSpec'] as Map).map((key, item) => MapEntry(key as String, item)),
    keepAlive: value['keepAlive'] as bool,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'containerPackageName': containerPackageName,
    'toolPkgId': toolPkgId,
    'routeId': routeId,
    'uiModuleId': uiModuleId,
    'runtime': runtime,
    'screen': screen,
    'title': title,
    'description': description,
    'moduleSpec': moduleSpec.map((key, item) => MapEntry(key, item)),
    'keepAlive': keepAlive,
  };

  final String containerPackageName;
  final String toolPkgId;
  final String routeId;
  final String uiModuleId;
  final String runtime;
  final String screen;
  final String title;
  final String description;
  final Map<String, Object?> moduleSpec;
  final bool keepAlive;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgWorkspaceTemplate`.
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

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgWorkspaceTemplate` from a MessagePack value map.
  factory ToolPkgWorkspaceTemplate.fromMessagePackValue(Map<String, Object?> value) => ToolPkgWorkspaceTemplate(
    containerPackageName: value['containerPackageName'] as String,
    toolPkgId: value['toolPkgId'] as String,
    templateId: value['templateId'] as String,
    displayName: value['displayName'] as String,
    description: value['description'] as String,
    resourceKey: value['resourceKey'] as String,
    projectType: value['projectType'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'containerPackageName': containerPackageName,
    'toolPkgId': toolPkgId,
    'templateId': templateId,
    'displayName': displayName,
    'description': description,
    'resourceKey': resourceKey,
    'projectType': projectType,
  };

  final String containerPackageName;
  final String toolPkgId;
  final String templateId;
  final String displayName;
  final String description;
  final String resourceKey;
  final String projectType;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAiProviderHandlerRuntime`.
class ToolPkgAiProviderHandlerRuntime {
  const ToolPkgAiProviderHandlerRuntime({
    required this.function,
    required this.functionSource,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAiProviderHandlerRuntime` from a MessagePack value map.
  factory ToolPkgAiProviderHandlerRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgAiProviderHandlerRuntime(
    function: value['function'] as String,
    functionSource: value['functionSource'] == null ? null : value['functionSource'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'function': function,
    'functionSource': functionSource == null ? null : functionSource!,
  };

  final String function;
  final String? functionSource;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAiProviderRuntime`.
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

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAiProviderRuntime` from a MessagePack value map.
  factory ToolPkgAiProviderRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgAiProviderRuntime(
    id: value['id'] as String,
    displayName: value['displayName'] as String,
    description: value['description'] as String,
    listModelsHandler: ToolPkgAiProviderHandlerRuntime.fromMessagePackValue(value['listModelsHandler'] as Map<String, Object?>),
    sendMessageHandler: ToolPkgAiProviderHandlerRuntime.fromMessagePackValue(value['sendMessageHandler'] as Map<String, Object?>),
    testConnectionHandler: ToolPkgAiProviderHandlerRuntime.fromMessagePackValue(value['testConnectionHandler'] as Map<String, Object?>),
    calculateInputTokensHandler: ToolPkgAiProviderHandlerRuntime.fromMessagePackValue(value['calculateInputTokensHandler'] as Map<String, Object?>),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'displayName': displayName,
    'description': description,
    'listModelsHandler': listModelsHandler.toMessagePackValue(),
    'sendMessageHandler': sendMessageHandler.toMessagePackValue(),
    'testConnectionHandler': testConnectionHandler.toMessagePackValue(),
    'calculateInputTokensHandler': calculateInputTokensHandler.toMessagePackValue(),
  };

  final String id;
  final String displayName;
  final String description;
  final ToolPkgAiProviderHandlerRuntime listModelsHandler;
  final ToolPkgAiProviderHandlerRuntime sendMessageHandler;
  final ToolPkgAiProviderHandlerRuntime testConnectionHandler;
  final ToolPkgAiProviderHandlerRuntime calculateInputTokensHandler;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAppLifecycleHookRuntime`.
class ToolPkgAppLifecycleHookRuntime {
  const ToolPkgAppLifecycleHookRuntime({
    required this.id,
    required this.event,
    required this.function,
    required this.functionSource,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgAppLifecycleHookRuntime` from a MessagePack value map.
  factory ToolPkgAppLifecycleHookRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgAppLifecycleHookRuntime(
    id: value['id'] as String,
    event: value['event'] as String,
    function: value['function'] as String,
    functionSource: value['functionSource'] == null ? null : value['functionSource'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'event': event,
    'function': function,
    'functionSource': functionSource == null ? null : functionSource!,
  };

  final String id;
  final String event;
  final String function;
  final String? functionSource;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgChatComposerSlotRuntime`.
class ToolPkgChatComposerSlotRuntime {
  const ToolPkgChatComposerSlotRuntime({
    required this.id,
    required this.slot,
    required this.screen,
    required this.order,
    required this.keepAlive,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgChatComposerSlotRuntime` from a MessagePack value map.
  factory ToolPkgChatComposerSlotRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgChatComposerSlotRuntime(
    id: value['id'] as String,
    slot: value['slot'] as String,
    screen: value['screen'] as String,
    order: (value['order'] as num).toInt(),
    keepAlive: value['keepAlive'] as bool,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'slot': slot,
    'screen': screen,
    'order': order,
    'keepAlive': keepAlive,
  };

  final String id;
  final String slot;
  final String screen;
  final int order;
  final bool keepAlive;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgChatMessageMenuDialogRuntime`.
class ToolPkgChatMessageMenuDialogRuntime {
  const ToolPkgChatMessageMenuDialogRuntime({
    required this.screen,
    required this.title,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgChatMessageMenuDialogRuntime` from a MessagePack value map.
  factory ToolPkgChatMessageMenuDialogRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgChatMessageMenuDialogRuntime(
    screen: value['screen'] as String,
    title: LocalizedText.fromMessagePackValue(value['title'] as Map<String, Object?>),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'screen': screen,
    'title': title.toMessagePackValue(),
  };

  final String screen;
  final LocalizedText title;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgChatMessageMenuItemRuntime`.
class ToolPkgChatMessageMenuItemRuntime {
  const ToolPkgChatMessageMenuItemRuntime({
    required this.id,
    required this.title,
    required this.icon,
    required this.order,
    required this.senders,
    required this.function,
    required this.functionSource,
    required this.dialog,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgChatMessageMenuItemRuntime` from a MessagePack value map.
  factory ToolPkgChatMessageMenuItemRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgChatMessageMenuItemRuntime(
    id: value['id'] as String,
    title: LocalizedText.fromMessagePackValue(value['title'] as Map<String, Object?>),
    icon: value['icon'] == null ? null : value['icon'] as String,
    order: (value['order'] as num).toInt(),
    senders: (value['senders'] as List<Object?>).map((item) => item as String).toList(growable: false),
    function: value['function'] as String,
    functionSource: value['functionSource'] == null ? null : value['functionSource'] as String,
    dialog: value['dialog'] == null ? null : ToolPkgChatMessageMenuDialogRuntime.fromMessagePackValue(value['dialog'] as Map<String, Object?>),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'title': title.toMessagePackValue(),
    'icon': icon == null ? null : icon!,
    'order': order,
    'senders': senders.map((item) => item).toList(growable: false),
    'function': function,
    'functionSource': functionSource == null ? null : functionSource!,
    'dialog': dialog == null ? null : dialog!.toMessagePackValue(),
  };

  final String id;
  final LocalizedText title;
  final String? icon;
  final int order;
  final List<String> senders;
  final String function;
  final String? functionSource;
  final ToolPkgChatMessageMenuDialogRuntime? dialog;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime`.
class ToolPkgContainerRuntime {
  const ToolPkgContainerRuntime({
    required this.packageName,
    required this.displayName,
    required this.description,
    required this.version,
    required this.apiVersion,
    required this.publicApi,
    required this.publicApis,
    required this.requires,
    required this.dependencyIssues,
    required this.manifestExtensions,
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
    required this.chatComposerSlots,
    required this.navigationEntries,
    required this.desktopWidgets,
    required this.appLifecycleHooks,
    required this.messageProcessingPlugins,
    required this.xmlRenderPlugins,
    required this.inputMenuTogglePlugins,
    required this.chatInputHooks,
    required this.chatViewHooks,
    required this.chatMessageHooks,
    required this.chatLifecycleHooks,
    required this.chatMessageMenuItems,
    required this.chatRuntimeHooks,
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
    required this.coreCommands,
    required this.aiProviders,
    required this.manifestExtensionHandlers,
    required this.logoResource,
    required this.marketOrigin,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime` from a MessagePack value map.
  factory ToolPkgContainerRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgContainerRuntime(
    packageName: value['packageName'] as String,
    displayName: LocalizedText.fromMessagePackValue(value['displayName'] as Map<String, Object?>),
    description: LocalizedText.fromMessagePackValue(value['description'] as Map<String, Object?>),
    version: value['version'] as String,
    apiVersion: value['apiVersion'] as String,
    publicApi: value['publicApi'] == null ? null : value['publicApi'] as String,
    publicApis: (value['publicApis'] as List<Object?>).map((item) => ToolPkgRegisteredFunctionHook.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    requires: (value['requires'] as List<Object?>).map((item) => ToolPkgManifestRequirement.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    dependencyIssues: (value['dependencyIssues'] as List<Object?>).map((item) => ToolPkgDependencyIssue.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    manifestExtensions: (value['manifestExtensions'] as Map).map((key, item) => MapEntry(key as String, item)),
    author: (value['author'] as List<Object?>).map((item) => item as String).toList(growable: false),
    mainEntry: value['mainEntry'] as String,
    sourceType: ToolPkgSourceType.fromMessagePackValue(value['sourceType'] as Map<String, Object?>),
    sourcePath: value['sourcePath'] as String,
    subpackages: (value['subpackages'] as List<Object?>).map((item) => ToolPkgSubpackageRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    resources: (value['resources'] as List<Object?>).map((item) => ToolPkgResourceRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    wasmModules: (value['wasmModules'] as List<Object?>).map((item) => ToolPkgWasmModuleRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    workflowTemplates: (value['workflowTemplates'] as List<Object?>).map((item) => ToolPkgWorkflowTemplateRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    workspaceTemplates: (value['workspaceTemplates'] as List<Object?>).map((item) => ToolPkgWorkspaceTemplateRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    uiModules: (value['uiModules'] as List<Object?>).map((item) => ToolPkgUiModuleRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    uiRoutes: (value['uiRoutes'] as List<Object?>).map((item) => ToolPkgUiRouteRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    chatComposerSlots: (value['chatComposerSlots'] as List<Object?>).map((item) => ToolPkgChatComposerSlotRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    navigationEntries: (value['navigationEntries'] as List<Object?>).map((item) => ToolPkgNavigationEntryRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    desktopWidgets: (value['desktopWidgets'] as List<Object?>).map((item) => ToolPkgDesktopWidgetRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    appLifecycleHooks: (value['appLifecycleHooks'] as List<Object?>).map((item) => ToolPkgAppLifecycleHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    messageProcessingPlugins: (value['messageProcessingPlugins'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    xmlRenderPlugins: (value['xmlRenderPlugins'] as List<Object?>).map((item) => ToolPkgTagFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    inputMenuTogglePlugins: (value['inputMenuTogglePlugins'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    chatInputHooks: (value['chatInputHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    chatViewHooks: (value['chatViewHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    chatMessageHooks: (value['chatMessageHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    chatLifecycleHooks: (value['chatLifecycleHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    chatMessageMenuItems: (value['chatMessageMenuItems'] as List<Object?>).map((item) => ToolPkgChatMessageMenuItemRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    chatRuntimeHooks: (value['chatRuntimeHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    hostEventHooks: (value['hostEventHooks'] as List<Object?>).map((item) => ToolPkgHostEventHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    toolLifecycleHooks: (value['toolLifecycleHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    promptInputHooks: (value['promptInputHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    promptHistoryHooks: (value['promptHistoryHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    promptEstimateHistoryHooks: (value['promptEstimateHistoryHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    systemPromptComposeHooks: (value['systemPromptComposeHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    toolPromptComposeHooks: (value['toolPromptComposeHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    promptFinalizeHooks: (value['promptFinalizeHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    promptEstimateFinalizeHooks: (value['promptEstimateFinalizeHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    summaryGenerateHooks: (value['summaryGenerateHooks'] as List<Object?>).map((item) => ToolPkgFunctionHookRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    coreCommands: (value['coreCommands'] as List<Object?>).map((item) => ToolPkgCoreCommandRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    aiProviders: (value['aiProviders'] as List<Object?>).map((item) => ToolPkgAiProviderRuntime.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    manifestExtensionHandlers: (value['manifestExtensionHandlers'] as List<Object?>).map((item) => ToolPkgRegisteredManifestExtension.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
    logoResource: value['logoResource'] == null ? null : ToolPkgResourceRuntime.fromMessagePackValue(value['logoResource'] as Map<String, Object?>),
    marketOrigin: value['marketOrigin'] == null ? null : ToolPkgMarketOrigin.fromMessagePackValue(value['marketOrigin'] as Map<String, Object?>),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'packageName': packageName,
    'displayName': displayName.toMessagePackValue(),
    'description': description.toMessagePackValue(),
    'version': version,
    'apiVersion': apiVersion,
    'publicApi': publicApi == null ? null : publicApi!,
    'publicApis': publicApis.map((item) => item.toMessagePackValue()).toList(growable: false),
    'requires': requires.map((item) => item.toMessagePackValue()).toList(growable: false),
    'dependencyIssues': dependencyIssues.map((item) => item.toMessagePackValue()).toList(growable: false),
    'manifestExtensions': manifestExtensions.map((key, item) => MapEntry(key, item)),
    'author': author.map((item) => item).toList(growable: false),
    'mainEntry': mainEntry,
    'sourceType': sourceType.toMessagePackValue(),
    'sourcePath': sourcePath,
    'subpackages': subpackages.map((item) => item.toMessagePackValue()).toList(growable: false),
    'resources': resources.map((item) => item.toMessagePackValue()).toList(growable: false),
    'wasmModules': wasmModules.map((item) => item.toMessagePackValue()).toList(growable: false),
    'workflowTemplates': workflowTemplates.map((item) => item.toMessagePackValue()).toList(growable: false),
    'workspaceTemplates': workspaceTemplates.map((item) => item.toMessagePackValue()).toList(growable: false),
    'uiModules': uiModules.map((item) => item.toMessagePackValue()).toList(growable: false),
    'uiRoutes': uiRoutes.map((item) => item.toMessagePackValue()).toList(growable: false),
    'chatComposerSlots': chatComposerSlots.map((item) => item.toMessagePackValue()).toList(growable: false),
    'navigationEntries': navigationEntries.map((item) => item.toMessagePackValue()).toList(growable: false),
    'desktopWidgets': desktopWidgets.map((item) => item.toMessagePackValue()).toList(growable: false),
    'appLifecycleHooks': appLifecycleHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'messageProcessingPlugins': messageProcessingPlugins.map((item) => item.toMessagePackValue()).toList(growable: false),
    'xmlRenderPlugins': xmlRenderPlugins.map((item) => item.toMessagePackValue()).toList(growable: false),
    'inputMenuTogglePlugins': inputMenuTogglePlugins.map((item) => item.toMessagePackValue()).toList(growable: false),
    'chatInputHooks': chatInputHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'chatViewHooks': chatViewHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'chatMessageHooks': chatMessageHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'chatLifecycleHooks': chatLifecycleHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'chatMessageMenuItems': chatMessageMenuItems.map((item) => item.toMessagePackValue()).toList(growable: false),
    'chatRuntimeHooks': chatRuntimeHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'hostEventHooks': hostEventHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'toolLifecycleHooks': toolLifecycleHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'promptInputHooks': promptInputHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'promptHistoryHooks': promptHistoryHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'promptEstimateHistoryHooks': promptEstimateHistoryHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'systemPromptComposeHooks': systemPromptComposeHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'toolPromptComposeHooks': toolPromptComposeHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'promptFinalizeHooks': promptFinalizeHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'promptEstimateFinalizeHooks': promptEstimateFinalizeHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'summaryGenerateHooks': summaryGenerateHooks.map((item) => item.toMessagePackValue()).toList(growable: false),
    'coreCommands': coreCommands.map((item) => item.toMessagePackValue()).toList(growable: false),
    'aiProviders': aiProviders.map((item) => item.toMessagePackValue()).toList(growable: false),
    'manifestExtensionHandlers': manifestExtensionHandlers.map((item) => item.toMessagePackValue()).toList(growable: false),
    'logoResource': logoResource == null ? null : logoResource!.toMessagePackValue(),
    'marketOrigin': marketOrigin == null ? null : marketOrigin!.toMessagePackValue(),
  };

  final String packageName;
  final LocalizedText displayName;
  final LocalizedText description;
  final String version;
  final String apiVersion;
  final String? publicApi;
  final List<ToolPkgRegisteredFunctionHook> publicApis;
  final List<ToolPkgManifestRequirement> requires;
  final List<ToolPkgDependencyIssue> dependencyIssues;
  final Map<String, Object?> manifestExtensions;
  final List<String> author;
  final String mainEntry;
  final ToolPkgSourceType sourceType;
  final String sourcePath;
  final List<ToolPkgSubpackageRuntime> subpackages;
  final List<ToolPkgResourceRuntime> resources;
  final List<ToolPkgWasmModuleRuntime> wasmModules;
  final List<ToolPkgWorkflowTemplateRuntime> workflowTemplates;
  final List<ToolPkgWorkspaceTemplateRuntime> workspaceTemplates;
  final List<ToolPkgUiModuleRuntime> uiModules;
  final List<ToolPkgUiRouteRuntime> uiRoutes;
  final List<ToolPkgChatComposerSlotRuntime> chatComposerSlots;
  final List<ToolPkgNavigationEntryRuntime> navigationEntries;
  final List<ToolPkgDesktopWidgetRuntime> desktopWidgets;
  final List<ToolPkgAppLifecycleHookRuntime> appLifecycleHooks;
  final List<ToolPkgFunctionHookRuntime> messageProcessingPlugins;
  final List<ToolPkgTagFunctionHookRuntime> xmlRenderPlugins;
  final List<ToolPkgFunctionHookRuntime> inputMenuTogglePlugins;
  final List<ToolPkgFunctionHookRuntime> chatInputHooks;
  final List<ToolPkgFunctionHookRuntime> chatViewHooks;
  final List<ToolPkgFunctionHookRuntime> chatMessageHooks;
  final List<ToolPkgFunctionHookRuntime> chatLifecycleHooks;
  final List<ToolPkgChatMessageMenuItemRuntime> chatMessageMenuItems;
  final List<ToolPkgFunctionHookRuntime> chatRuntimeHooks;
  final List<ToolPkgHostEventHookRuntime> hostEventHooks;
  final List<ToolPkgFunctionHookRuntime> toolLifecycleHooks;
  final List<ToolPkgFunctionHookRuntime> promptInputHooks;
  final List<ToolPkgFunctionHookRuntime> promptHistoryHooks;
  final List<ToolPkgFunctionHookRuntime> promptEstimateHistoryHooks;
  final List<ToolPkgFunctionHookRuntime> systemPromptComposeHooks;
  final List<ToolPkgFunctionHookRuntime> toolPromptComposeHooks;
  final List<ToolPkgFunctionHookRuntime> promptFinalizeHooks;
  final List<ToolPkgFunctionHookRuntime> promptEstimateFinalizeHooks;
  final List<ToolPkgFunctionHookRuntime> summaryGenerateHooks;
  final List<ToolPkgCoreCommandRuntime> coreCommands;
  final List<ToolPkgAiProviderRuntime> aiProviders;
  final List<ToolPkgRegisteredManifestExtension> manifestExtensionHandlers;
  final ToolPkgResourceRuntime? logoResource;
  final ToolPkgMarketOrigin? marketOrigin;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgCoreCommandRuntime`.
class ToolPkgCoreCommandRuntime {
  const ToolPkgCoreCommandRuntime({
    required this.id,
    required this.name,
    required this.title,
    required this.description,
    required this.usage,
    required this.function,
    required this.functionSource,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgCoreCommandRuntime` from a MessagePack value map.
  factory ToolPkgCoreCommandRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgCoreCommandRuntime(
    id: value['id'] as String,
    name: value['name'] as String,
    title: LocalizedText.fromMessagePackValue(value['title'] as Map<String, Object?>),
    description: LocalizedText.fromMessagePackValue(value['description'] as Map<String, Object?>),
    usage: value['usage'] as String,
    function: value['function'] as String,
    functionSource: value['functionSource'] == null ? null : value['functionSource'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'name': name,
    'title': title.toMessagePackValue(),
    'description': description.toMessagePackValue(),
    'usage': usage,
    'function': function,
    'functionSource': functionSource == null ? null : functionSource!,
  };

  final String id;
  final String name;
  final LocalizedText title;
  final LocalizedText description;
  final String usage;
  final String function;
  final String? functionSource;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgDependencyIssue`.
class ToolPkgDependencyIssue {
  const ToolPkgDependencyIssue({
    required this.id,
    required this.code,
    required this.requiredMinVersion,
    required this.requiredMaxVersion,
    required this.installedVersion,
    required this.enabled,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgDependencyIssue` from a MessagePack value map.
  factory ToolPkgDependencyIssue.fromMessagePackValue(Map<String, Object?> value) => ToolPkgDependencyIssue(
    id: value['id'] as String,
    code: value['code'] as String,
    requiredMinVersion: value['requiredMinVersion'] == null ? null : value['requiredMinVersion'] as String,
    requiredMaxVersion: value['requiredMaxVersion'] == null ? null : value['requiredMaxVersion'] as String,
    installedVersion: value['installedVersion'] == null ? null : value['installedVersion'] as String,
    enabled: value['enabled'] as bool,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'code': code,
    'requiredMinVersion': requiredMinVersion == null ? null : requiredMinVersion!,
    'requiredMaxVersion': requiredMaxVersion == null ? null : requiredMaxVersion!,
    'installedVersion': installedVersion == null ? null : installedVersion!,
    'enabled': enabled,
  };

  final String id;
  final String code;
  final String? requiredMinVersion;
  final String? requiredMaxVersion;
  final String? installedVersion;
  final bool enabled;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgDesktopWidgetRuntime`.
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

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgDesktopWidgetRuntime` from a MessagePack value map.
  factory ToolPkgDesktopWidgetRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgDesktopWidgetRuntime(
    id: value['id'] as String,
    routeId: value['routeId'] as String,
    renderRouteId: value['renderRouteId'] as String,
    title: LocalizedText.fromMessagePackValue(value['title'] as Map<String, Object?>),
    subtitle: LocalizedText.fromMessagePackValue(value['subtitle'] as Map<String, Object?>),
    description: LocalizedText.fromMessagePackValue(value['description'] as Map<String, Object?>),
    icon: value['icon'] == null ? null : value['icon'] as String,
    order: (value['order'] as num).toInt(),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'routeId': routeId,
    'renderRouteId': renderRouteId,
    'title': title.toMessagePackValue(),
    'subtitle': subtitle.toMessagePackValue(),
    'description': description.toMessagePackValue(),
    'icon': icon == null ? null : icon!,
    'order': order,
  };

  final String id;
  final String routeId;
  final String renderRouteId;
  final LocalizedText title;
  final LocalizedText subtitle;
  final LocalizedText description;
  final String? icon;
  final int order;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgFunctionHookRuntime`.
class ToolPkgFunctionHookRuntime {
  const ToolPkgFunctionHookRuntime({
    required this.id,
    required this.function,
    required this.functionSource,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgFunctionHookRuntime` from a MessagePack value map.
  factory ToolPkgFunctionHookRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgFunctionHookRuntime(
    id: value['id'] as String,
    function: value['function'] as String,
    functionSource: value['functionSource'] == null ? null : value['functionSource'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'function': function,
    'functionSource': functionSource == null ? null : functionSource!,
  };

  final String id;
  final String function;
  final String? functionSource;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgHostEventHookRuntime`.
class ToolPkgHostEventHookRuntime {
  const ToolPkgHostEventHookRuntime({
    required this.id,
    required this.source,
    required this.trigger,
    required this.function,
    required this.functionSource,
    required this.enabled,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgHostEventHookRuntime` from a MessagePack value map.
  factory ToolPkgHostEventHookRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgHostEventHookRuntime(
    id: value['id'] as String,
    source: value['source'] as String,
    trigger: value['trigger'],
    function: value['function'] as String,
    functionSource: value['functionSource'] == null ? null : value['functionSource'] as String,
    enabled: value['enabled'] as bool,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'source': source,
    'trigger': trigger,
    'function': function,
    'functionSource': functionSource == null ? null : functionSource!,
    'enabled': enabled,
  };

  final String id;
  final String source;
  final Object? trigger;
  final String function;
  final String? functionSource;
  final bool enabled;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgManifestRequirement`.
class ToolPkgManifestRequirement {
  const ToolPkgManifestRequirement({
    required this.id,
    required this.description,
    required this.minVersion,
    required this.maxVersion,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgManifestRequirement` from a MessagePack value map.
  factory ToolPkgManifestRequirement.fromMessagePackValue(Map<String, Object?> value) => ToolPkgManifestRequirement(
    id: value['id'] as String,
    description: value['description'] as String,
    minVersion: value['min_version'] == null ? null : value['min_version'] as String,
    maxVersion: value['max_version'] == null ? null : value['max_version'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'description': description,
    'min_version': minVersion == null ? null : minVersion!,
    'max_version': maxVersion == null ? null : maxVersion!,
  };

  final String id;
  final String description;
  final String? minVersion;
  final String? maxVersion;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgMarketOrigin`.
class ToolPkgMarketOrigin {
  const ToolPkgMarketOrigin({
    required this.market,
    required this.toolpkgId,
    required this.version,
    required this.author,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgMarketOrigin` from a MessagePack value map.
  factory ToolPkgMarketOrigin.fromMessagePackValue(Map<String, Object?> value) => ToolPkgMarketOrigin(
    market: value['market'] as String,
    toolpkgId: value['toolpkgId'] as String,
    version: value['version'] as String,
    author: (value['author'] as List<Object?>).map((item) => item as String).toList(growable: false),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'market': market,
    'toolpkgId': toolpkgId,
    'version': version,
    'author': author.map((item) => item).toList(growable: false),
  };

  final String market;
  final String toolpkgId;
  final String version;
  final List<String> author;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgNavigationActionHookRuntime`.
class ToolPkgNavigationActionHookRuntime {
  const ToolPkgNavigationActionHookRuntime({
    required this.function,
    required this.functionSource,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgNavigationActionHookRuntime` from a MessagePack value map.
  factory ToolPkgNavigationActionHookRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgNavigationActionHookRuntime(
    function: value['function'] as String,
    functionSource: value['functionSource'] == null ? null : value['functionSource'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'function': function,
    'functionSource': functionSource == null ? null : functionSource!,
  };

  final String function;
  final String? functionSource;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgNavigationEntryRuntime`.
class ToolPkgNavigationEntryRuntime {
  const ToolPkgNavigationEntryRuntime({
    required this.id,
    required this.routeId,
    required this.params,
    required this.surface,
    required this.title,
    required this.action,
    required this.icon,
    required this.order,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgNavigationEntryRuntime` from a MessagePack value map.
  factory ToolPkgNavigationEntryRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgNavigationEntryRuntime(
    id: value['id'] as String,
    routeId: value['routeId'] as String,
    params: value['params'],
    surface: value['surface'] as String,
    title: LocalizedText.fromMessagePackValue(value['title'] as Map<String, Object?>),
    action: value['action'] == null ? null : ToolPkgNavigationActionHookRuntime.fromMessagePackValue(value['action'] as Map<String, Object?>),
    icon: value['icon'] == null ? null : value['icon'] as String,
    order: (value['order'] as num).toInt(),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'routeId': routeId,
    'params': params,
    'surface': surface,
    'title': title.toMessagePackValue(),
    'action': action == null ? null : action!.toMessagePackValue(),
    'icon': icon == null ? null : icon!,
    'order': order,
  };

  final String id;
  final String routeId;
  final Object? params;
  final String surface;
  final LocalizedText title;
  final ToolPkgNavigationActionHookRuntime? action;
  final String? icon;
  final int order;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgRegisteredFunctionHook`.
class ToolPkgRegisteredFunctionHook {
  const ToolPkgRegisteredFunctionHook({
    required this.id,
    required this.function,
    required this.functionSource,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgRegisteredFunctionHook` from a MessagePack value map.
  factory ToolPkgRegisteredFunctionHook.fromMessagePackValue(Map<String, Object?> value) => ToolPkgRegisteredFunctionHook(
    id: value['id'] as String,
    function: value['function'] as String,
    functionSource: value['functionSource'] == null ? null : value['functionSource'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'function': function,
    'functionSource': functionSource == null ? null : functionSource!,
  };

  final String id;
  final String function;
  final String? functionSource;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgRegisteredManifestExtension`.
class ToolPkgRegisteredManifestExtension {
  const ToolPkgRegisteredManifestExtension({
    required this.key,
    required this.function,
    required this.functionSource,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgRegisteredManifestExtension` from a MessagePack value map.
  factory ToolPkgRegisteredManifestExtension.fromMessagePackValue(Map<String, Object?> value) => ToolPkgRegisteredManifestExtension(
    key: value['key'] as String,
    function: value['function'] as String,
    functionSource: value['functionSource'] == null ? null : value['functionSource'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'key': key,
    'function': function,
    'functionSource': functionSource == null ? null : functionSource!,
  };

  final String key;
  final String function;
  final String? functionSource;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgResourceRuntime`.
class ToolPkgResourceRuntime {
  const ToolPkgResourceRuntime({
    required this.key,
    required this.path,
    required this.mime,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgResourceRuntime` from a MessagePack value map.
  factory ToolPkgResourceRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgResourceRuntime(
    key: value['key'] as String,
    path: value['path'] as String,
    mime: value['mime'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'key': key,
    'path': path,
    'mime': mime,
  };

  final String key;
  final String path;
  final String mime;
}

enum ToolPkgSourceType {
  ASSET,
  MARKET,
  EXTERNAL,
  ;

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgSourceType` from its MessagePack scalar value.
  factory ToolPkgSourceType.fromMessagePackValue(Object? value) => switch (value) {
    'ASSET' => ToolPkgSourceType.ASSET,
    'MARKET' => ToolPkgSourceType.MARKET,
    'EXTERNAL' => ToolPkgSourceType.EXTERNAL,
    _ => throw ArgumentError('Unknown ToolPkgSourceType: $value'),
  };

  /// Encodes this enum into a MessagePack scalar value.
  String toMessagePackValue() => switch (this) {
    ToolPkgSourceType.ASSET => 'ASSET',
    ToolPkgSourceType.MARKET => 'MARKET',
    ToolPkgSourceType.EXTERNAL => 'EXTERNAL',
  };
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgSubpackageRuntime`.
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

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgSubpackageRuntime` from a MessagePack value map.
  factory ToolPkgSubpackageRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgSubpackageRuntime(
    packageName: value['packageName'] as String,
    containerPackageName: value['containerPackageName'] as String,
    subpackageId: value['subpackageId'] as String,
    entryPath: value['entryPath'] as String,
    displayName: LocalizedText.fromMessagePackValue(value['displayName'] as Map<String, Object?>),
    description: LocalizedText.fromMessagePackValue(value['description'] as Map<String, Object?>),
    enabledByDefault: value['enabledByDefault'] as bool,
    toolCount: (value['toolCount'] as num).toInt(),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'packageName': packageName,
    'containerPackageName': containerPackageName,
    'subpackageId': subpackageId,
    'entryPath': entryPath,
    'displayName': displayName.toMessagePackValue(),
    'description': description.toMessagePackValue(),
    'enabledByDefault': enabledByDefault,
    'toolCount': toolCount,
  };

  final String packageName;
  final String containerPackageName;
  final String subpackageId;
  final String entryPath;
  final LocalizedText displayName;
  final LocalizedText description;
  final bool enabledByDefault;
  final int toolCount;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgTagFunctionHookRuntime`.
class ToolPkgTagFunctionHookRuntime {
  const ToolPkgTagFunctionHookRuntime({
    required this.id,
    required this.tag,
    required this.function,
    required this.functionSource,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgTagFunctionHookRuntime` from a MessagePack value map.
  factory ToolPkgTagFunctionHookRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgTagFunctionHookRuntime(
    id: value['id'] as String,
    tag: value['tag'] as String,
    function: value['function'] as String,
    functionSource: value['functionSource'] == null ? null : value['functionSource'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'tag': tag,
    'function': function,
    'functionSource': functionSource == null ? null : functionSource!,
  };

  final String id;
  final String tag;
  final String function;
  final String? functionSource;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgUiModuleRuntime`.
class ToolPkgUiModuleRuntime {
  const ToolPkgUiModuleRuntime({
    required this.id,
    required this.runtime,
    required this.screen,
    required this.title,
    required this.keepAlive,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgUiModuleRuntime` from a MessagePack value map.
  factory ToolPkgUiModuleRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgUiModuleRuntime(
    id: value['id'] as String,
    runtime: value['runtime'] as String,
    screen: value['screen'] as String,
    title: LocalizedText.fromMessagePackValue(value['title'] as Map<String, Object?>),
    keepAlive: value['keepAlive'] as bool,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'runtime': runtime,
    'screen': screen,
    'title': title.toMessagePackValue(),
    'keepAlive': keepAlive,
  };

  final String id;
  final String runtime;
  final String screen;
  final LocalizedText title;
  final bool keepAlive;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgUiRouteRuntime`.
class ToolPkgUiRouteRuntime {
  const ToolPkgUiRouteRuntime({
    required this.id,
    required this.routeId,
    required this.runtime,
    required this.screen,
    required this.title,
    required this.keepAlive,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgUiRouteRuntime` from a MessagePack value map.
  factory ToolPkgUiRouteRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgUiRouteRuntime(
    id: value['id'] as String,
    routeId: value['routeId'] as String,
    runtime: value['runtime'] as String,
    screen: value['screen'] as String,
    title: LocalizedText.fromMessagePackValue(value['title'] as Map<String, Object?>),
    keepAlive: value['keepAlive'] as bool,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'routeId': routeId,
    'runtime': runtime,
    'screen': screen,
    'title': title.toMessagePackValue(),
    'keepAlive': keepAlive,
  };

  final String id;
  final String routeId;
  final String runtime;
  final String screen;
  final LocalizedText title;
  final bool keepAlive;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgWasmModuleRuntime`.
class ToolPkgWasmModuleRuntime {
  const ToolPkgWasmModuleRuntime({
    required this.id,
    required this.path,
    required this.exports,
    required this.sourceLanguage,
    required this.abi,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgWasmModuleRuntime` from a MessagePack value map.
  factory ToolPkgWasmModuleRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgWasmModuleRuntime(
    id: value['id'] as String,
    path: value['path'] as String,
    exports: (value['exports'] as List<Object?>).map((item) => item as String).toList(growable: false),
    sourceLanguage: value['sourceLanguage'] as String,
    abi: value['abi'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'path': path,
    'exports': exports.map((item) => item).toList(growable: false),
    'sourceLanguage': sourceLanguage,
    'abi': abi,
  };

  final String id;
  final String path;
  final List<String> exports;
  final String sourceLanguage;
  final String abi;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgTemplateModels::ToolPkgWorkflowTemplateRuntime`.
class ToolPkgWorkflowTemplateRuntime {
  const ToolPkgWorkflowTemplateRuntime({
    required this.id,
    required this.display_name,
    required this.description,
    required this.resource_key,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgTemplateModels::ToolPkgWorkflowTemplateRuntime` from a MessagePack value map.
  factory ToolPkgWorkflowTemplateRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgWorkflowTemplateRuntime(
    id: value['id'] as String,
    display_name: LocalizedText.fromMessagePackValue(value['display_name'] as Map<String, Object?>),
    description: LocalizedText.fromMessagePackValue(value['description'] as Map<String, Object?>),
    resource_key: value['resource_key'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'display_name': display_name.toMessagePackValue(),
    'description': description.toMessagePackValue(),
    'resource_key': resource_key,
  };

  final String id;
  final LocalizedText display_name;
  final LocalizedText description;
  final String resource_key;
}

/// Generated SDK model for Rust type `operit_plugin_sdk::toolpkg::ToolPkgTemplateModels::ToolPkgWorkspaceTemplateRuntime`.
class ToolPkgWorkspaceTemplateRuntime {
  const ToolPkgWorkspaceTemplateRuntime({
    required this.id,
    required this.display_name,
    required this.description,
    required this.resource_key,
    required this.project_type,
  });

  /// Decodes `operit_plugin_sdk::toolpkg::ToolPkgTemplateModels::ToolPkgWorkspaceTemplateRuntime` from a MessagePack value map.
  factory ToolPkgWorkspaceTemplateRuntime.fromMessagePackValue(Map<String, Object?> value) => ToolPkgWorkspaceTemplateRuntime(
    id: value['id'] as String,
    display_name: LocalizedText.fromMessagePackValue(value['display_name'] as Map<String, Object?>),
    description: LocalizedText.fromMessagePackValue(value['description'] as Map<String, Object?>),
    resource_key: value['resource_key'] as String,
    project_type: value['project_type'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'display_name': display_name.toMessagePackValue(),
    'description': description.toMessagePackValue(),
    'resource_key': resource_key,
    'project_type': project_type,
  };

  final String id;
  final LocalizedText display_name;
  final LocalizedText description;
  final String resource_key;
  final String project_type;
}

/// Generated SDK model for Rust type `operit_tools::ConversationMarkupManager::ToolResult`.
class ToolResult {
  const ToolResult({
    required this.toolName,
    required this.success,
    required this.result,
    required this.error,
  });

  /// Decodes `operit_tools::ConversationMarkupManager::ToolResult` from a MessagePack value map.
  factory ToolResult.fromMessagePackValue(Map<String, Object?> value) => ToolResult(
    toolName: value['toolName'] as String,
    success: value['success'] as bool,
    result: value['result'],
    error: value['error'] == null ? null : value['error'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'toolName': toolName,
    'success': success,
    'result': result,
    'error': error == null ? null : error!,
  };

  final String toolName;
  final bool success;
  final Object? result;
  final String? error;
}

/// Generated SDK model for Rust type `operit_tools::tools::PackageLoadingProgress::PluginLoadingItem`.
class PluginLoadingItem {
  const PluginLoadingItem({
    required this.id,
    required this.displayName,
    required this.kind,
    required this.status,
    required this.message,
    required this.logText,
  });

  /// Decodes `operit_tools::tools::PackageLoadingProgress::PluginLoadingItem` from a MessagePack value map.
  factory PluginLoadingItem.fromMessagePackValue(Map<String, Object?> value) => PluginLoadingItem(
    id: value['id'] as String,
    displayName: value['displayName'] as String,
    kind: value['kind'] as String,
    status: value['status'] as String,
    message: value['message'] as String,
    logText: value['logText'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'id': id,
    'displayName': displayName,
    'kind': kind,
    'status': status,
    'message': message,
    'logText': logText,
  };

  final String id;
  final String displayName;
  final String kind;
  final String status;
  final String message;
  final String logText;
}

/// Generated SDK model for Rust type `operit_tools::tools::PackageLoadingProgress::PluginLoadingProgress`.
class PluginLoadingProgress {
  const PluginLoadingProgress({
    required this.visible,
    required this.forceExpanded,
    required this.progress,
    required this.phase,
    required this.currentTask,
    required this.pluginsStarted,
    required this.pluginsTotal,
    required this.plugins,
  });

  /// Decodes `operit_tools::tools::PackageLoadingProgress::PluginLoadingProgress` from a MessagePack value map.
  factory PluginLoadingProgress.fromMessagePackValue(Map<String, Object?> value) => PluginLoadingProgress(
    visible: value['visible'] as bool,
    forceExpanded: value['forceExpanded'] as bool,
    progress: (value['progress'] as num).toDouble(),
    phase: value['phase'] as String,
    currentTask: value['currentTask'] as String,
    pluginsStarted: (value['pluginsStarted'] as num).toInt(),
    pluginsTotal: (value['pluginsTotal'] as num).toInt(),
    plugins: (value['plugins'] as List<Object?>).map((item) => PluginLoadingItem.fromMessagePackValue(item as Map<String, Object?>)).toList(growable: false),
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'visible': visible,
    'forceExpanded': forceExpanded,
    'progress': progress,
    'phase': phase,
    'currentTask': currentTask,
    'pluginsStarted': pluginsStarted,
    'pluginsTotal': pluginsTotal,
    'plugins': plugins.map((item) => item.toMessagePackValue()).toList(growable: false),
  };

  final bool visible;
  final bool forceExpanded;
  final double progress;
  final String phase;
  final String currentTask;
  final int pluginsStarted;
  final int pluginsTotal;
  final List<PluginLoadingItem> plugins;
}

/// Generated SDK model for Rust type `operit_tools::tools::packTool::RuntimePackageManager::ToolPkgLoadIssue`.
class ToolPkgLoadIssue {
  const ToolPkgLoadIssue({
    required this.sourcePath,
    required this.packageName,
    required this.displayName,
    required this.code,
    required this.message,
    required this.packageKind,
  });

  /// Decodes `operit_tools::tools::packTool::RuntimePackageManager::ToolPkgLoadIssue` from a MessagePack value map.
  factory ToolPkgLoadIssue.fromMessagePackValue(Map<String, Object?> value) => ToolPkgLoadIssue(
    sourcePath: value['sourcePath'] as String,
    packageName: value['packageName'] == null ? null : value['packageName'] as String,
    displayName: value['displayName'] as String,
    code: value['code'] as String,
    message: value['message'] as String,
    packageKind: value['packageKind'] as String,
  );

  /// Encodes this model into a MessagePack-compatible value map.
  Map<String, Object?> toMessagePackValue() => <String, Object?>{
    'sourcePath': sourcePath,
    'packageName': packageName == null ? null : packageName!,
    'displayName': displayName,
    'code': code,
    'message': message,
    'packageKind': packageKind,
  };

  final String sourcePath;
  final String? packageName;
  final String displayName;
  final String code;
  final String message;
  final String packageKind;
}

