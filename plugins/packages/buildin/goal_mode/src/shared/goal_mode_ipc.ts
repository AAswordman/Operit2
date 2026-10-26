import {
  readSingleActiveChatView,
  removeTrackedChatView,
  setGoal,
  setGoalStatus,
  markGoalMessageReviewed,
  isGoalRuntime,
  upsertTrackedChatView,
  type GoalRecord,
  type GoalRuntime,
  type GoalStatus,
  type GoalTrackedChatView,
} from "./goal_mode_state.js";
import {
  deletePersistedGoal, readPersistedGoal, writePersistedGoal,
  isGoalInputSlotEnabled, setGoalInputSlotEnabled, watchPersistedGoal,
} from "./goal_mode_persistence.js";

export const GOAL_MODE_ACTIVE_VIEW_CHANNEL = "goal_mode.active_view";
export const GOAL_MODE_TRACK_VIEW_CHANNEL = "goal_mode.track_view";
export const GOAL_MODE_REMOVE_VIEW_CHANNEL = "goal_mode.remove_view";
const GOAL_MODE_MAIN_CONTEXT_KEY = "toolpkg_main:com.operit.goal_mode_bundle";

/** Returns the main execution context used by Goal Mode state handlers. */
function goalModeIpcOptions(): ToolPkg.IpcCallOptions {
  return { targetContextKey: GOAL_MODE_MAIN_CONTEXT_KEY };
}

/** Registers the shared goal-state IPC endpoints in the owning plugin runtime. */
export function registerGoalModeIpc(): void {
  ToolPkg.ipc.on<void, GoalTrackedChatView | null>(
    GOAL_MODE_ACTIVE_VIEW_CHANNEL,
    readSingleActiveChatView
  );
  ToolPkg.ipc.on<GoalTrackedChatView, void>(GOAL_MODE_TRACK_VIEW_CHANNEL, upsertTrackedChatView);
  ToolPkg.ipc.on<{ runtime: GoalRuntime; viewId: string }, void>(
    GOAL_MODE_REMOVE_VIEW_CHANNEL,
    (request) => {
      if (!isGoalRuntime(request.runtime)) {
        throw new Error("Goal view runtime is invalid.");
      }
      removeTrackedChatView(request.runtime, request.viewId);
    }
  );

}

/** Reads the active chat view through the owning runtime. */
export async function readSingleActiveChatViewAsync(): Promise<GoalTrackedChatView | null> {
  return await ToolPkg.ipc.call<void, GoalTrackedChatView | null>(
    GOAL_MODE_ACTIVE_VIEW_CHANNEL,
    undefined,
    goalModeIpcOptions()
  );
}

/** Tracks a chat view through the owning runtime. */
export async function upsertTrackedChatViewAsync(view: GoalTrackedChatView): Promise<void> {
  await ToolPkg.ipc.call<GoalTrackedChatView, void>(
    GOAL_MODE_TRACK_VIEW_CHANNEL,
    view,
    goalModeIpcOptions()
  );
}

/** Removes a closed chat view through the owning runtime. */
export async function removeTrackedChatViewAsync(runtime: GoalRuntime, viewId: string): Promise<void> {
  await ToolPkg.ipc.call<{ runtime: GoalRuntime; viewId: string }, void>(
    GOAL_MODE_REMOVE_VIEW_CHANNEL,
    { runtime, viewId },
    goalModeIpcOptions()
  );
}

/** Implements routed goal operations using this package's own synchronized storage. */
export class GoalService {
  /** Reads the current durable goal. */
  @ToolPkg.route({ binding: 0 })
  static async read(chatId: string): Promise<GoalRecord | null> { return readPersistedGoal(chatId); }
  /** Stores a newly established goal. */
  @ToolPkg.route({ binding: 0 })
  static async set(chatId: string, objective: string): Promise<GoalRecord> { return writePersistedGoal(chatId, setGoal(chatId, objective)); }
  /** Applies a status change to the current durable goal. */
  @ToolPkg.route({ binding: 0 })
  static async status(chatId: string, status: GoalStatus): Promise<GoalRecord | null> {
    await readPersistedGoal(chatId);
    const goal = setGoalStatus(chatId, status);
    return goal ? writePersistedGoal(chatId, goal) : null;
  }
  /** Saves the assistant message review checkpoint. */
  @ToolPkg.route({ binding: 0 })
  static async reviewed(chatId: string, timestamp: number): Promise<GoalRecord | null> {
    await readPersistedGoal(chatId);
    const goal = markGoalMessageReviewed(chatId, timestamp);
    return goal ? writePersistedGoal(chatId, goal) : null;
  }
  /** Deletes the durable goal. */
  @ToolPkg.route({ binding: 0 })
  static async clear(chatId: string): Promise<void> { await deletePersistedGoal(chatId); }
  /** Reads the synchronized input flag. */
  @ToolPkg.route({ binding: 0 })
  static async inputEnabled(chatId: string): Promise<boolean> { return isGoalInputSlotEnabled(chatId); }
  /** Updates the synchronized input flag. */
  @ToolPkg.route({ binding: 0 })
  static async setInput(chatId: string, enabled: boolean): Promise<void> { await setGoalInputSlotEnabled(chatId, enabled); }
  /** Streams the current goal and all subsequent synchronized changes. */
  @ToolPkg.route({ binding: 0 })
  static async *watch(chatId: string, signal?: AbortSignal): AsyncGenerator<GoalRecord | null> {
    if (!signal) throw new Error("Goal stream requires its collector cancellation signal");
    yield* watchPersistedGoal(chatId, signal);
  }
}
