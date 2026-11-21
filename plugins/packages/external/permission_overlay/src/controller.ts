export const PACKAGE = "com.operit.permission_overlay";
export const OVERLAY_ROUTE = `toolpkg:${PACKAGE}:ui:approval`;
export const SETTINGS_ROUTE = `toolpkg:${PACKAGE}:ui:settings`;
/** Presents the oldest approval; the host owns window lifecycle and notification ordering. */
export async function onPermissionsChanged(event: ToolPkg.ToolPermissionHookEvent): Promise<void> {
  const requests = event.eventPayload.requests;
  await ToolPkg.ui.floatingWindows.set("tool-permission", requests.length === 0 ? null : {
    route: OVERLAY_ROUTE,
    fill: true,
    state: { request: requests[0], pendingCount: requests.length },
  });
}
