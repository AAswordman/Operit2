export const INDICATOR_ROUTE = "toolpkg:com.operit.ui_automation:ui:indicator";

const WINDOW_ID = "automation-activity";
const UI_TOOLS = [
  "list_windows", "focus_window", "get_page_info", "tap", "long_press",
  "click_element", "set_input_text", "press_key", "swipe",
];
// Packaged tools await beginOperation through IPC before sending device input.
// Only native tools need asynchronous lifecycle notifications here.
const AUTOMATION_TOOLS = new Set(UI_TOOLS);

const activeChats = new Set<string>();
let visibility: "unknown" | "foreground" | "background" | "detached" = "unknown";
let pending: Promise<void> = Promise.resolve();

/** Serializes state and host requests across simultaneous chats and lifecycle events. */
function enqueue(action: () => Promise<void>): Promise<void> {
  const result = pending.then(action);
  pending = result.catch(() => {});
  return result;
}

/** Starts or refreshes this chat's indicator before a packaged operation runs. */
export function beginOperation(chatId: string): Promise<void> {
  return enqueue(async () => {
    if (!chatId?.trim()) throw new Error("UI operation requires a chat context");
    if (visibility === "detached") return;
    activeChats.add(chatId);
    await reconcile();
  });
}

/** Releases this chat and awaits window cleanup, including retries after host errors. */
export function completeOperation(chatId: string): Promise<void> {
  return enqueue(async () => {
    if (!chatId?.trim()) throw new Error("UI operation requires a chat context");
    activeChats.delete(chatId);
    await reconcile();
  });
}

/** Reconciles the indicator within the actual host scope without prompting for permission. */
async function reconcile(): Promise<void> {
  const windows = ToolPkg.ui.floatingWindows;
  const owned = (await windows.list()).some(window => window.id === WINDOW_ID);
  let show = false;
  if (activeChats.size && visibility !== "detached") {
    const caps = await windows.capabilities();
    show = caps.supported && caps.permissionGranted && caps.clickThrough && (
      caps.scope === "system" ||
      (caps.scope === "application" && visibility !== "background")
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
  if (!chatId || !AUTOMATION_TOOLS.has(toolName)) return;
  await beginOperation(chatId);
}

/** Releases only the conversation that completed, failed or was stopped. */
export async function onChatState(event: ToolPkg.ChatRuntimeHookEvent): Promise<void> {
  const { chatId, isActive } = event.eventPayload;
  if (!isActive) await completeOperation(chatId);
}

/** Updates presentation when the shared app lifecycle changes. */
export async function onAppLifecycle(event: ToolPkg.AppLifecycleHookEvent): Promise<void> {
  const next = event.eventName === "application_on_foreground" ? "foreground" :
    event.eventName === "application_on_background" ? "background" : "detached";
  await enqueue(async () => {
    visibility = next;
    if (next === "detached") activeChats.clear();
    await reconcile();
  });
}
