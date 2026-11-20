import { onAppLifecycle, onChatState, onToolLifecycle, INDICATOR_ROUTE } from "./indicator";
import { indicatorScreen, statusScreen } from "./ui";

export { onAppLifecycle, onChatState, onToolLifecycle, indicatorScreen, statusScreen };

const STATUS_ROUTE = "toolpkg:com.operit.ui_automation:ui:status";

/** Registers the plugin-owned surfaces and their runtime hooks. */
export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({
    id: "indicator", route: INDICATOR_ROUTE, runtime: "compose_dsl", screen: indicatorScreen,
    title: { zh: "自动化运行提示", en: "Automation activity" },
  });
  const title = { zh: "UI 自动化", en: "UI Automation" };
  ToolPkg.registerUiRoute({
    id: "status", route: STATUS_ROUTE, runtime: "compose_dsl", screen: statusScreen, title,
  });
  ToolPkg.registerNavigationEntry({
    id: "status", route: STATUS_ROUTE, surface: "toolbox", icon: "TouchApp", title,
  });
  for (const event of ["application_on_foreground", "application_on_background", "application_on_terminate"] as const) {
    ToolPkg.registerAppLifecycleHook({ id: event, event, function: onAppLifecycle });
  }
  ToolPkg.registerToolLifecycleHook({ id: "track-automation", function: onToolLifecycle });
  ToolPkg.registerChatRuntimeHook({ id: "finish-automation", function: onChatState });
  return true;
}
