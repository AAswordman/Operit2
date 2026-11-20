export const INDICATOR_ROUTE = "toolpkg:com.operit.ui_automation:ui:indicator";

const WINDOW_ID = "automation-activity";
const AUTOMATION_TOOLS = new Set([
  "list_windows", "focus_window", "get_page_info", "tap", "long_press",
  "click_element", "set_input_text", "press_key", "swipe",
]);
type Bounds = { left: number; top: number; right: number; bottom: number };
type Surface = { id: string; bounds?: Bounds; visible: boolean };
const activeChats = new Map<string, string | undefined>();
const surfaces = new Map<string, Surface>();
let serial = 0;
let visibility: "unknown" | "foreground" | "background" | "detached" = "unknown";
let pending: Promise<void> = Promise.resolve();
let refresh: ReturnType<typeof setTimeout> | undefined;
let querying = false;

/** Serializes chat changes, target refreshes and host requests. */
function enqueue(action: () => Promise<void>): Promise<void> {
  const result = pending.then(action);
  pending = result.catch(() => {});
  return result;
}

/** Keeps the chosen window across subsequent coordinate/key operations. */
export function beginOperation(chatId: string, windowId?: string): Promise<void> {
  return enqueue(async () => {
    if (!chatId?.trim()) throw new Error("UI operation requires a chat context");
    if (visibility === "detached") return;
    activeChats.set(chatId, windowId?.trim() || activeChats.get(chatId));
    await reconcile();
  });
}

/** Releases only this chat's target, including retries after host errors. */
export function completeOperation(chatId: string): Promise<void> {
  return enqueue(async () => {
    if (!chatId?.trim()) throw new Error("UI operation requires a chat context");
    activeChats.delete(chatId);
    await reconcile();
  });
}

/** Parses native coordinates without converting physical pixels in JavaScript. */
function parseBounds(raw?: string): Bounds | undefined {
  const match = raw?.match(/^\[(-?\d+(?:\.\d+)?),(-?\d+(?:\.\d+)?)\]\[(-?\d+(?:\.\d+)?),(-?\d+(?:\.\d+)?)\]$/);
  if (!match) return;
  const [left, top, right, bottom] = match.slice(1).map(Number);
  if (![left, top, right, bottom].every(value => Number.isFinite(value) && Math.abs(value) <= 1000000) || right <= left || bottom <= top) return;
  return { left, top, right, bottom };
}

/** Uses one non-overlapping refresh loop only while system windows are targeted. */
function scheduleRefresh(enabled: boolean): void {
  if (!enabled && refresh !== undefined) { clearTimeout(refresh); refresh = undefined; }
  if (!enabled || refresh !== undefined) return;
  refresh = setTimeout(() => {
    refresh = undefined;
    void enqueue(reconcile).catch(error => {
      console.warn("Unable to refresh automation indicator", error);
      scheduleRefresh(activeChats.size > 0 && visibility !== "detached");
    });
  }, 350);
}

/** Follows selected windows; unselected operations retain a full-scope indicator. */
async function reconcile(): Promise<void> {
  const windows = ToolPkg.ui.floatingWindows;
  const owned = await windows.list();
  const caps = activeChats.size ? await windows.capabilities() : undefined;
  const show = caps?.supported && caps.permissionGranted && caps.clickThrough && visibility !== "detached" && (
    caps.scope === "system" || (caps.scope === "application" && visibility !== "background")
  );
  const targets = new Set(show ? (caps?.scope === "system" ? activeChats.values() : [undefined]) : []);
  scheduleRefresh(!!show && caps?.scope === "system" && [...targets].some(target => target !== undefined));

  let native: Awaited<ReturnType<typeof Tools.UI.listWindows>>["windows"] = [];
  if ([...targets].some(target => target !== undefined)) {
    querying = true;
    try { native = (await Tools.UI.listWindows()).windows; }
    catch { /* An unavailable provider hides the target instead of marking a different window. */ }
    finally { querying = false; }
  }

  // Recover after a main-runtime restart without adopting another plugin surface.
  const tracked = new Set([...surfaces.values()].map(surface => surface.id));
  for (const window of owned) {
    if ((window.id === WINDOW_ID || window.id.startsWith(`${WINDOW_ID}-window-`)) && !tracked.has(window.id)) {
      await windows.close(window.id);
    }
  }
  const desired = new Set([...targets].map(target => target ?? ""));
  for (const [target, surface] of surfaces) {
    if (!desired.has(target)) {
      if (owned.some(window => window.id === surface.id)) await windows.close(surface.id);
      surfaces.delete(target);
    }
  }
  for (const target of targets) {
    const key = target ?? "";
    const selected = target === undefined ? undefined : native.find(window => window.windowId === target);
    const bounds = selected && !selected.isMinimized ? parseBounds(selected.bounds) : undefined;
    let surface = surfaces.get(key);
    if (surface && !owned.some(window => window.id === surface!.id)) {
      surfaces.delete(key); surface = undefined;
    }
    if (target !== undefined && !bounds) {
      if (surface?.visible) { await windows.hide(surface.id); surface.visible = false; }
      continue;
    }
    if (!surface) {
      const id = target === undefined ? WINDOW_ID : `${WINDOW_ID}-window-${++serial}`;
      await windows.open({
        id, route: INDICATOR_ROUTE,
        ...(bounds ? {} : { fill: true }),
        options: { clickThrough: true, focusable: false, ...(bounds ? { screenBounds: bounds } : {}) },
      });
      surfaces.set(key, { id, bounds, visible: true });
    } else {
      if (bounds && JSON.stringify(bounds) !== JSON.stringify(surface.bounds)) {
        await windows.update(surface.id, { screenBounds: bounds });
        surface.bounds = bounds;
      }
      if (!surface.visible) { await windows.show(surface.id); surface.visible = true; }
    }
  }
}

/** Native tool calls also propagate their explicit window selection. */
export async function onToolLifecycle(event: ToolPkg.ToolLifecycleHookEvent): Promise<void> {
  if (event.eventName !== "tool_execution_started" || visibility === "detached") return;
  const { toolName, runtimeContext, parameters } = event.eventPayload;
  if (querying && toolName === "list_windows") return;
  const chatId = runtimeContext?.chatId;
  if (!chatId || !AUTOMATION_TOOLS.has(toolName)) return;
  await beginOperation(chatId, toolName === "focus_window" || toolName === "get_page_info" ? parameters?.window_id : undefined);
}

export async function onChatState(event: ToolPkg.ChatRuntimeHookEvent): Promise<void> {
  const { chatId, isActive } = event.eventPayload;
  if (!isActive) await completeOperation(chatId);
}

export async function onAppLifecycle(event: ToolPkg.AppLifecycleHookEvent): Promise<void> {
  const next = event.eventName === "application_on_foreground" ? "foreground" :
    event.eventName === "application_on_background" ? "background" : "detached";
  await enqueue(async () => {
    visibility = next;
    if (next === "detached") activeChats.clear();
    await reconcile();
  });
}
