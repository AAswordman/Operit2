import "../../plugins/types/index";

/** Checks that plugin registration is available only through the injected ToolPkg object. */
function verifyRegistrationNamespace(): void {
  void ToolPkg.registerToolboxUiModule;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgToolboxUiModule;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerToolboxUiModule;
  void ToolPkg.registerUiRoute;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgUiRoute;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerUiRoute;
  void ToolPkg.registerChatComposerSlot;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgChatComposerSlot;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerChatComposerSlot;
  void ToolPkg.registerNavigationEntry;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgNavigationEntry;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerNavigationEntry;
  void ToolPkg.registerDesktopWidget;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgDesktopWidget;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerDesktopWidget;
  void ToolPkg.registerAppLifecycleHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgAppLifecycleHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerAppLifecycleHook;
  void ToolPkg.registerMessageProcessingPlugin;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgMessageProcessingPlugin;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerMessageProcessingPlugin;
  void ToolPkg.registerXmlRenderPlugin;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgXmlRenderPlugin;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerXmlRenderPlugin;
  void ToolPkg.registerInputMenuTogglePlugin;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgInputMenuTogglePlugin;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerInputMenuTogglePlugin;
  void ToolPkg.registerChatInputHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgChatInputHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerChatInputHook;
  void ToolPkg.registerChatViewHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgChatViewHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerChatViewHook;
  void ToolPkg.registerChatMessageHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgChatMessageHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerChatMessageHook;
  void ToolPkg.registerChatLifecycleHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgChatLifecycleHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerChatLifecycleHook;
  void ToolPkg.registerChatMessageMenuItem;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgChatMessageMenuItem;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerChatMessageMenuItem;
  void ToolPkg.registerChatRuntimeHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgChatRuntimeHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerChatRuntimeHook;
  void ToolPkg.registerHostEventHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgHostEventHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerHostEventHook;
  void ToolPkg.registerToolLifecycleHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgToolLifecycleHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerToolLifecycleHook;
  void ToolPkg.registerPromptInputHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgPromptInputHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerPromptInputHook;
  void ToolPkg.registerPromptHistoryHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgPromptHistoryHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerPromptHistoryHook;
  void ToolPkg.registerPromptEstimateHistoryHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgPromptEstimateHistoryHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerPromptEstimateHistoryHook;
  void ToolPkg.registerSystemPromptComposeHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgSystemPromptComposeHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerSystemPromptComposeHook;
  void ToolPkg.registerToolPromptComposeHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgToolPromptComposeHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerToolPromptComposeHook;
  void ToolPkg.registerPromptFinalizeHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgPromptFinalizeHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerPromptFinalizeHook;
  void ToolPkg.registerPromptEstimateFinalizeHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgPromptEstimateFinalizeHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerPromptEstimateFinalizeHook;
  void ToolPkg.registerSummaryGenerateHook;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgSummaryGenerateHook;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerSummaryGenerateHook;
  void ToolPkg.registerCoreCommand;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgCoreCommand;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerCoreCommand;
  void ToolPkg.registerAiProvider;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgAiProvider;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerAiProvider;
  void ToolPkg.registerManifestExtension;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgManifestExtension;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerManifestExtension;
  void ToolPkg.registerApi;
  // @ts-expect-error Standalone registration aliases are not part of the SDK.
  void registerToolPkgApi;
  // @ts-expect-error Short global registration aliases are not part of the SDK.
  void registerApi;
}

void verifyRegistrationNamespace;
