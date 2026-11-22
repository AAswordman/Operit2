import { beginOperation, completeOperation, onAppLifecycle, onChatState, onToolLifecycle, INDICATOR_ROUTE } from "./indicator";
import { indicatorScreen } from "./ui";

export { onAppLifecycle, onChatState, onToolLifecycle, indicatorScreen };

// Registration capture and runtime execution use different engines. IPC loads
// this module without invoking registerToolPkg, so handlers belong at module scope.
ToolPkg.ipc.on<{ chatId: string; windowId?: string | null }, void>("ui_automation.begin", ({ chatId, windowId }) => beginOperation(chatId, windowId));
ToolPkg.ipc.on<{ chatId: string; closeWindows?: boolean }, void>("ui_automation.complete", ({ chatId, closeWindows }) => completeOperation(chatId, closeWindows));

/** Registers the plugin-owned surfaces and their runtime hooks. */
export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({
    id: "indicator", route: INDICATOR_ROUTE, runtime: "compose_dsl", screen: indicatorScreen,
    title: { zh: "自动化运行提示", en: "Automation activity" },
  });
  for (const event of ["application_on_foreground", "application_on_background", "application_on_terminate"] as const) {
    ToolPkg.registerAppLifecycleHook({ id: event, event, function: onAppLifecycle });
  }
  ToolPkg.registerToolLifecycleHook({ id: "track-automation", function: onToolLifecycle });
  ToolPkg.registerChatRuntimeHook({ id: "finish-automation", function: onChatState });
  return true;
}
