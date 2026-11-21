import { onPermissionsChanged, OVERLAY_ROUTE, SETTINGS_ROUTE } from "./controller";
import { approvalScreen, settingsScreen } from "./ui";

export { onPermissionsChanged, approvalScreen, settingsScreen };

/** Registers the optional approval presenter and its settings entry. */
export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({ id: "approval", route: OVERLAY_ROUTE, runtime: "compose_dsl", screen: approvalScreen,
    title: { zh: "权限请求", en: "Permission request" } });
  ToolPkg.registerUiRoute({ id: "settings", route: SETTINGS_ROUTE, runtime: "compose_dsl", screen: settingsScreen,
    title: { zh: "权限悬浮窗", en: "Permission Overlay" } });
  ToolPkg.registerNavigationEntry({ id: "settings", route: SETTINGS_ROUTE, surface: "toolbox", icon: "Shield",
    title: { zh: "权限悬浮窗", en: "Permission Overlay" } });
  ToolPkg.registerToolPermissionHook({ id: "approval-overlay", function: onPermissionsChanged });
  return true;
}
