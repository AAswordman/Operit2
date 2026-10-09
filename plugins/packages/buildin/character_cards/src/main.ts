import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import { registerUiRequestChannel, registerServiceLifecycle, register, renderScreen } from "./host";
import { registerDomainCommands } from "./commands";
import { registerDomainApis } from "./public-api";
import { connectDirectorySources } from "./service-runtime";
import { registerUiContributionApis } from "./ui-contributions";
import { registerMemoryJobHooks } from "./memory-jobs/hooks";
import { registerToolPolicies } from "./runtime-tools/policy";
import { registerChatInitialization } from "./chat-lifecycle";
import { renderGroupExecutionScreen } from "./group-execution/control";
import { registerGroupExecutionHooks } from "./group-execution/hooks";
import { registerSidebarChannel } from "./ui-sidebar";
import { renderSidebarScreen } from "./ui-sidebar";
import { renderSelectionScreen } from "./ui-selector";
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
/** Declares IPC handlers when the main runtime is evaluated; no business service is opened. */
registerUiRequestChannel();
/** Registers the actual embedded-sidebar catalog IPC without reading host metadata or opening the business store. */
registerSidebarChannel();
/** Renders character cards with their bound memories in one Material interface. */
export function screen(ctx: ComposeDslContext): ComposeNode { return renderScreen(ctx, definition); }
/** Renders the independent attachment route through the same typed offline UI and result channel. */
export function attachmentScreen(ctx: ComposeDslContext): ComposeNode { return renderScreen(ctx, definition, "memory-attachment"); }
/** Renders only the independently registered embedded sidebar, with no presentation session or management inference. */
export function sidebarScreen(ctx: ComposeDslContext): ComposeNode { return renderSidebarScreen(ctx); }
/** Renders the native compact role selector separately from the large Web editor and sidebar. */
export function selectionScreen(ctx: ComposeDslContext): ComposeNode { return renderSelectionScreen(ctx); }
/** Renders the package-owned submission controls through the existing generic presentation route. */
export function groupExecutionScreen(ctx: ComposeDslContext): ComposeNode { return renderGroupExecutionScreen(ctx); }
/** Registers plugin-owned domain commands, typed public APIs, and independent character/memory/sidebar surfaces. */
export function registerToolPkg(): boolean {
  registerSelectionSettingsAccess();
  registerDomainCommands();
  registerDomainApis();
  registerServiceLifecycle();
  registerMemoryJobHooks();
  registerToolPolicies();
  registerChatInitialization();
  registerGroupExecutionHooks();
  registerUiContributionApis();
  return register(definition, screen, attachmentScreen, sidebarScreen, selectionScreen, groupExecutionScreen);
}
