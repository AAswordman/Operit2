export type GoalStatus = "active" | "paused" | "completed";
export type GoalRuntime = "main" | "floating";

/** Checks whether a value identifies one supported chat runtime surface. */
export function isGoalRuntime(value: unknown): value is GoalRuntime {
  return value === "main" || value === "floating";
}

export type GoalRecord = {
  objective: string;
  status: GoalStatus;
  updatedAt: number;
  startedAt: number;
  pausedAt?: number;
  pausedDurationMs: number;
  completedAt?: number;
  lastReviewedMessageTimestamp?: number;
};

export type GoalTrackedChatView = {
  viewId: string;
  runtime: GoalRuntime;
  chatId: string;
  title: string;
  updatedAt: number;
};

const goalsByChatId: Record<string, GoalRecord> = {};
const trackedViewsByChatId: Record<string, GoalTrackedChatView> = {};

/** Returns an isolated copy of one goal record. */
function cloneGoal(goal: GoalRecord): GoalRecord {
  return {
    objective: goal.objective,
    status: goal.status,
    updatedAt: goal.updatedAt,
    startedAt: goal.startedAt,
    pausedAt: goal.pausedAt,
    pausedDurationMs: goal.pausedDurationMs,
    completedAt: goal.completedAt,
    lastReviewedMessageTimestamp: goal.lastReviewedMessageTimestamp,
  };
}

/** Returns an isolated copy of one tracked chat view. */
function cloneTrackedView(view: GoalTrackedChatView): GoalTrackedChatView {
  return {
    viewId: view.viewId,
    runtime: view.runtime,
    chatId: view.chatId,
    title: view.title,
    updatedAt: view.updatedAt,
  };
}

/** Reads the goal associated with a chat. */
export function readGoal(chatId: string): GoalRecord | null {
  const goal = goalsByChatId[chatId];
  return goal ? cloneGoal(goal) : null;
}

/** Stores one active goal for a chat. */
export function setGoal(chatId: string, objective: string): GoalRecord {
  const now = Date.now();
  const goal: GoalRecord = {
    objective,
    status: "active",
    updatedAt: now,
    startedAt: now,
    pausedDurationMs: 0,
  };
  goalsByChatId[chatId] = goal;
  return cloneGoal(goal);
}

/** Restores a persisted goal into the in-memory goal state. */
export function restoreGoal(chatId: string, goal: GoalRecord): GoalRecord {
  goalsByChatId[chatId] = cloneGoal(goal);
  return cloneGoal(goal);
}

/** Changes the status of an existing goal. */
export function setGoalStatus(chatId: string, status: GoalStatus): GoalRecord | null {
  const goal = goalsByChatId[chatId];
  if (!goal) {
    return null;
  }
  const now = Date.now();
  if (status === "paused" && goal.status === "active") {
    goal.pausedAt = now;
  }
  if (status === "active" && goal.status === "paused" && goal.pausedAt !== undefined) {
    goal.pausedDurationMs += Math.max(0, now - goal.pausedAt);
    delete goal.pausedAt;
  }
  if (status === "active" && goal.status === "completed") {
    delete goal.completedAt;
  }
  if (status === "completed") {
    if (goal.pausedAt !== undefined) {
      goal.pausedDurationMs += Math.max(0, now - goal.pausedAt);
      delete goal.pausedAt;
    }
    goal.completedAt = now;
  }
  goal.status = status;
  goal.updatedAt = now;
  return cloneGoal(goal);
}

/** Records the assistant message timestamp that the goal reviewer handled. */
export function markGoalMessageReviewed(chatId: string, timestamp: number): GoalRecord | null {
  const goal = goalsByChatId[chatId];
  if (!goal) {
    return null;
  }
  goal.lastReviewedMessageTimestamp = timestamp;
  goal.updatedAt = Date.now();
  return cloneGoal(goal);
}

/** Removes a goal from a chat. */
export function clearGoal(chatId: string): void {
  delete goalsByChatId[chatId];
}

/** Records the currently active chat view. */
export function upsertTrackedChatView(view: GoalTrackedChatView): void {
  trackedViewsByChatId[view.chatId] = cloneTrackedView(view);
}

/** Removes a closed chat view by its host identity. */
export function removeTrackedChatView(runtime: GoalRuntime, viewId: string): void {
  Object.entries(trackedViewsByChatId).forEach(([chatId, view]) => {
    if (view.runtime === runtime && view.viewId === viewId) {
      delete trackedViewsByChatId[chatId];
    }
  });
}

/** Reads the tracked chat view associated with a chat. */
export function readTrackedChatViewByChatId(chatId: string): GoalTrackedChatView | null {
  const view = trackedViewsByChatId[chatId];
  return view ? cloneTrackedView(view) : null;
}

/** Reads the most recently updated tracked chat view. */
export function readSingleActiveChatView(): GoalTrackedChatView | null {
  const views = Object.values(trackedViewsByChatId);
  if (views.length === 0) {
    return null;
  }
  return cloneTrackedView(
    views.reduce((latest, view) => view.updatedAt > latest.updatedAt ? view : latest)
  );
}
