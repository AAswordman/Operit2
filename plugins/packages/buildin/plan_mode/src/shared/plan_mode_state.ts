export type PlanModeRuntime = "main" | "floating";

export type PlanModeTrackedChatView = {
  viewId: string;
  runtime: PlanModeRuntime;
  chatId: string;
  title: string;
  updatedAt: number;
};

type PlanModeState = {
  trackedViewsByChatId: Record<string, PlanModeTrackedChatView>;
};

const state: PlanModeState = {
  trackedViewsByChatId: {},
};

/** Clones a tracked view before it crosses the state boundary. */
function cloneTrackedChatView(view: PlanModeTrackedChatView): PlanModeTrackedChatView {
  return {
    viewId: view.viewId,
    runtime: view.runtime,
    chatId: view.chatId,
    title: view.title,
    updatedAt: view.updatedAt,
  };
}

/** Clones every tracked chat view into a new lookup map. */
function cloneTrackedViewsByChatId(): Record<string, PlanModeTrackedChatView> {
  const next: Record<string, PlanModeTrackedChatView> = {};
  Object.entries(state.trackedViewsByChatId).forEach(([chatId, view]) => {
    next[chatId] = cloneTrackedChatView(view);
  });
  return next;
}

/** Returns an isolated snapshot of all in-memory plan state. */
export function readPlanModeStateSnapshot(): PlanModeState {
  return {
    trackedViewsByChatId: cloneTrackedViewsByChatId(),
  };
}

/** Reads plan state through the async shared-state contract. */
export async function readPlanModeStateAsync(): Promise<PlanModeState> {
  return readPlanModeStateSnapshot();
}

/** Returns the tracked chat view active in one runtime. */
export function readActiveChatViewForRuntime(runtime: PlanModeRuntime): PlanModeTrackedChatView | null {
  const view = Object.values(state.trackedViewsByChatId).find((item) => item.runtime === runtime);
  return view ? cloneTrackedChatView(view) : null;
}

/** Returns the tracked view associated with one chat. */
export function readTrackedChatViewByChatId(chatId: string): PlanModeTrackedChatView | null {
  const view = state.trackedViewsByChatId[chatId];
  return view ? cloneTrackedChatView(view) : null;
}

/** Returns the chat view most recently reported by the host runtime. */
export function readSingleActiveChatView(): PlanModeTrackedChatView | null {
  const views = Object.values(state.trackedViewsByChatId);
  if (views.length === 0) {
    return null;
  }
  const activeView = views.reduce((latest, view) =>
    view.updatedAt > latest.updatedAt ? view : latest
  );
  return cloneTrackedChatView(activeView);
}

/** Stores the latest chat view and replaces the view in its runtime. */
export async function upsertTrackedChatViewAsync(view: PlanModeTrackedChatView): Promise<void> {
  state.trackedViewsByChatId[view.chatId] = cloneTrackedChatView(view);
}

/** Removes a tracked view when its matching runtime view closes. */
export async function removeTrackedChatViewAsync(
  runtime: PlanModeRuntime,
  viewId: string
): Promise<void> {
  Object.entries(state.trackedViewsByChatId).forEach(([chatId, trackedView]) => {
    if (trackedView.runtime === runtime && trackedView.viewId === viewId) {
      delete state.trackedViewsByChatId[chatId];
    }
  });
}
