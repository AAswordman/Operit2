import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import {
  GoalService,
} from "../../shared/goal_mode_ipc.js";
import type { GoalRecord } from "../../shared/goal_mode_state.js";

/** Calculates active goal time from its persisted timing checkpoints. */
function calculateGoalElapsedMs(goal: GoalRecord, now: number): number {
  const end = goal.pausedAt ?? goal.completedAt ?? now;
  return Math.max(0, end - goal.startedAt - goal.pausedDurationMs);
}

/** Formats an elapsed duration as a fixed-width hours, minutes, and seconds value. */
function formatGoalDuration(durationMs: number): string {
  const totalSeconds = Math.floor(durationMs / 1000);
  const hours = Math.floor(totalSeconds / 3600).toString().padStart(2, "0");
  const minutes = Math.floor((totalSeconds % 3600) / 60).toString().padStart(2, "0");
  const seconds = (totalSeconds % 60).toString().padStart(2, "0");
  return `${hours}:${minutes}:${seconds}`;
}

/** Renders the current goal and its state controls above the chat composer. */
export default function Screen(ctx: ComposeDslContext): ComposeNode {
  const [chatId] = ctx.useState<string>("chatId", "");
  const [expanded, setExpanded] = ctx.useState("goal.expanded", false);
  const [goal, setGoal] = ctx.useState<GoalRecord | null>("goal", null);
  const goalRef = ctx.useRef<GoalRecord | null>("currentGoal", goal);
  goalRef.current = goal;
  /** Owns the subscription and elapsed-time refresh for this exact chat context. */
  ctx.useEffect("goal", () => {
    setGoal(null);
    const iterator = GoalService.watch(chatId);
    let active = true;
    /** Consumes routed state until the composition releases this collector. */
    const observe = async (): Promise<void> => {
      for await (const next of iterator) {
        if (!active) return;
        setGoal(next);
      }
    };
    void observe().catch((error) => { if (active) ctx.reportError(error); });
    /** Releases the chat subscription and timer together. */
    return () => {
      active = false;
      void iterator.return(undefined).catch((error) => { ctx.reportError(error); });
    };
  }, [chatId]);
  /** Refreshes elapsed time only while an active goal exposes its clock. */
  ctx.useEffect("goal-clock", () => {
    if (goal?.status !== "active" || !expanded) return () => {};
    const timer = setInterval(() => {
      if (goalRef.current?.status === "active") setGoal({ ...goalRef.current });
    }, 1000);
    /** Releases this composition's elapsed-time timer. */
    return () => { clearInterval(timer); };
  }, [chatId, goal?.status, expanded]);

  /** Pauses the durable goal on its Binding owner. */
  const pause = async (): Promise<void> => { await GoalService.status(chatId, "paused"); };
  /** Resumes the durable goal on its Binding owner. */
  const resume = async (): Promise<void> => { await GoalService.status(chatId, "active"); };
  /** Removes the durable goal on its Binding owner. */
  const clear = async (): Promise<void> => { await GoalService.clear(chatId); };

  const active = goal?.status === "active";

  // Keep the empty slot zero-height while the initial read is pending too.
  // Transient padding here resizes the transcript and nudges its bottom twice.
  if (goal === null) {
    return ctx.UI.Column({ fillMaxWidth: true }, []);
  }

  const paused = goal.status === "paused";
  const title = active ? "目标进行中" : paused ? "目标已暂停" : "目标已完成";
  const elapsed = formatGoalDuration(calculateGoalElapsedMs(goal, Date.now()));
  /** Changes only the amount of goal context shown above the composer. */
  const toggleExpanded = (): void => { setExpanded(!expanded); };

  return ctx.UI.Column(
    { key: "goal.composer", fillMaxWidth: true, paddingHorizontal: 0, paddingBottom: 8 },
    [ctx.UI.Card(
      { key: "goal.surface", fillMaxWidth: true, containerColor: "surfaceContainer", themeSurface: "card", shape: { cornerRadius: 8 }, elevation: 0 },
      ctx.UI.Column({ fillMaxWidth: true }, [
        ctx.UI.Row(
          { key: "goal.header", fillMaxWidth: true, padding: { horizontal: 12, vertical: 4 }, spacing: 4, verticalAlignment: "center", onClick: toggleExpanded },
          [
            ctx.UI.Icon({ key: "goal.status-icon", name: "flag", tint: "onSurfaceVariant", size: 16 }),
            ctx.UI.Text({ key: "goal.title", text: goal.objective, style: "labelMedium", color: "onSurfaceVariant", maxLines: 1, overflow: "ellipsis", weight: 1 }),
            ...(active ? [ctx.UI.IconButton({ key: "goal.pause", content: ctx.UI.Icon({ name: "pause", size: 20 }), tooltip: "暂停目标", width: 32, height: 32, onClick: pause })]
              : paused ? [ctx.UI.IconButton({ key: "goal.resume", content: ctx.UI.Icon({ name: "playArrow", size: 20 }), tooltip: "继续目标", width: 32, height: 32, onClick: resume })]
              : [ctx.UI.Box({ width: 32, height: 32, contentAlignment: "center" }, ctx.UI.Icon({ name: "checkCircle", tint: "onSurfaceVariant", size: 16 }))]),
            ctx.UI.IconButton({ key: "goal.clear", content: ctx.UI.Icon({ name: "delete", size: 20 }), tooltip: "清除目标", width: 32, height: 32, onClick: clear }),
            ctx.UI.IconButton({ key: "goal.expand", content: ctx.UI.Icon({ name: expanded ? "keyboardArrowUp" : "keyboardArrowDown", size: 20 }), tooltip: expanded ? "收起目标" : "展开目标", width: 32, height: 32, onClick: toggleExpanded }),
          ],
        ),
        ...(expanded ? [ctx.UI.Column({ key: "goal.details", fillMaxWidth: true, paddingHorizontal: 12, paddingBottom: 8, spacing: 4 }, [
          ctx.UI.Text({ text: title + " · " + elapsed, style: "labelSmall", color: "onSurfaceVariant" }),
          ctx.UI.Text({ text: goal.objective, style: "bodySmall", maxLines: 4, overflow: "ellipsis" }),
        ])] : []),
      ]),
    )],
  );
}
