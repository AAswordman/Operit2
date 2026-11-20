export const INDICATOR_ROUTE = "toolpkg:com.operit.ui_automation:ui:indicator";

const WINDOW_ID = "automation-activity";
const UI_TOOLS = [
  "list_windows", "focus_window", "get_page_info", "tap", "long_press",
  "click_element", "set_input_text", "press_key", "swipe",
];
const AUTOMATION_TOOLS = new Set([
  ...UI_TOOLS,
  ...[...UI_TOOLS, "app_launch", "double_tap", "get_page_screenshot_image"]
    .map(name => `Automatic_ui_base:${name}`),
]);

const activeChats = new Set<string>();
let visibility: "unknown" | "foreground" | "background" | "detached" = "unknown";

/** Reconciles the indicator within the actual host scope without prompting for permission. */
async function reconcile(): Promise<void> {
  if (visibility === "unknown") return;
  const windows = ToolPkg.ui.floatingWindows;
  const owned = (await windows.list()).some(window => window.id === WINDOW_ID);
  let show = false;
  if (activeChats.size && visibility !== "detached") {
    const caps = await windows.capabilities();
    show = caps.supported && caps.permissionGranted && caps.clickThrough && (
      (caps.scope === "system" && visibility === "background") ||
      (caps.scope === "application" && visibility === "foreground")
    );
  }
  if (show && !owned) {
    await windows.open({
      id: WINDOW_ID,
      route: INDICATOR_ROUTE,
      fill: true,
      options: { clickThrough: true, focusable: false },
    });
  } else if (!show && owned) {
    await windows.close(WINDOW_ID);
  }
}

/** Keeps an automation turn marked across its reads, actions and model round trips. */
export async function onToolLifecycle(event: ToolPkg.ToolLifecycleHookEvent): Promise<void> {
  if (event.eventName !== "tool_execution_started" || visibility === "detached") return;
  const { toolName, runtimeContext } = event.eventPayload;
  const chatId = runtimeContext?.chatId;
  if (!chatId || !AUTOMATION_TOOLS.has(toolName) || activeChats.has(chatId)) return;
  activeChats.add(chatId);
  if (activeChats.size === 1) await reconcile();
}

/** Releases only the conversation that completed, failed or was stopped. */
export async function onChatState(event: ToolPkg.ChatRuntimeHookEvent): Promise<void> {
  const { chatId, isActive } = event.eventPayload;
  if (!isActive && activeChats.delete(chatId) && activeChats.size === 0) await reconcile();
}

/** Updates presentation when the shared app lifecycle changes. */
export async function onAppLifecycle(event: ToolPkg.AppLifecycleHookEvent): Promise<void> {
  const next = event.eventName === "application_on_foreground" ? "foreground" :
    event.eventName === "application_on_background" ? "background" : "detached";
  if (visibility === next) return;
  visibility = next;
  if (activeChats.size === 0) return;
  if (next === "detached") activeChats.clear();
  await reconcile();
}
