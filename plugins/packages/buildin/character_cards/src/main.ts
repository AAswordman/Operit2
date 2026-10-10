import memoryScreen from "./ui/memory/index.ui.js";
import screen from "./ui/main/index.ui.js";
import attachmentScreen from "./ui/memory-attachment/index.ui.js";
import sidebarScreen from "./ui/chat-sidebar/index.ui.js";
import selectionScreen from "./ui/selection/index.ui.js";
import inputMenuScreen from "./ui/chat-input-menu/index.ui.js";
import groupExecutionScreen from "./ui/group-execution/index.ui.js";
import { registerUiRequestChannel, registerServiceLifecycle, register } from "./host";
import { registerDomainCommands } from "./commands";
import { registerDomainApis } from "./public-api";
import { connectDirectorySources } from "./service-runtime";
import { registerUiContributionApis } from "./ui-contributions";
import { registerMemoryJobHooks } from "./memory-jobs/hooks";
import { registerToolPolicies } from "./runtime-tools/policy";
import { registerChatInitialization } from "./chat-lifecycle";
import { registerGroupExecutionHooks } from "./group-execution/hooks";
import { registerSidebarChannel } from "./ui-sidebar";
import { registerSelectionSettingsAccess } from "./selection-settings";
export * from "./commands";
export * from "./public-api";
export { onServiceInitialize } from "./host";
export { onMemoryMessagePersisted, onMemoryInterval } from "./memory-jobs/hooks";
export { toolCallPolicy, toolPromptPolicy } from "./runtime-tools/policy";
export { beforeChatCreate } from "./chat-lifecycle";
export { onGroupInputSubmit } from "./group-execution/hooks";
export { chatContextActionsApi, chatListSectionsApi } from "./ui-contributions";
import { definition } from "./definition";
/** Connects lazy directory readers for every evaluated runtime, not just the registration-only module. */
connectDirectorySources({
  /** Reads the actual configured model directory only when the shared service requests it. */
  listModels: () => Tools.SoftwareSettings.listModelSummaries(),
  /** Reads the actual TTS directory without opening storage during registration. */
  listTtsConfigs: () => Tools.SoftwareSettings.listTtsConfigs(),
  /** Reads complete real builtin, package, skill and MCP sources through the generic settings capability. */
  readToolCatalog: () => Tools.SoftwareSettings.readToolSourceCatalog(),
});
/** Connects independent configuration capabilities in every evaluated execution runtime. */
registerSelectionSettingsAccess();
/** Declares IPC handlers when the main runtime is evaluated; no business service is opened. */
registerUiRequestChannel();
/** Registers the actual embedded-sidebar catalog IPC without reading host metadata or opening the business store. */
registerSidebarChannel();
/** Registers plugin-owned domain commands, typed public APIs, and independent character/memory/sidebar surfaces. */
export function registerToolPkg(): boolean {
  registerDomainCommands();
  registerDomainApis();
  registerServiceLifecycle();
  registerMemoryJobHooks();
  registerToolPolicies();
  registerChatInitialization();
  registerGroupExecutionHooks();
  registerUiContributionApis();
  return register(definition, screen, attachmentScreen, sidebarScreen, selectionScreen, groupExecutionScreen, memoryScreen, inputMenuScreen);
}
