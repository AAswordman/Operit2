import { claimPlanStart, forgetPlanStart, isPlanStarted, watchPlanStarted } from "./plan_mode_handoff.js";
import {
  disablePlanMode,
  enablePlanModeForChat,
  isPlanModeEnabledForChat,
} from "./plan_mode_mode.js";
import {
  hasPlanFile,
  watchPlanFile,
  writePlanFile,
  type PlanFileRecord,
} from "./plan_mode_plan_file.js";
import {
  readSingleActiveChatView,
  readTrackedChatViewByChatId,
  removeTrackedChatViewAsync,
  upsertTrackedChatViewAsync,
  type PlanModeRuntime,
  type PlanModeTrackedChatView,
} from "./plan_mode_state.js";

/** Shares device UI view bookkeeping through the existing main-context IPC boundary. */
function Shared(channel: string) {
  /** Registers the implemented method through a standard TypeScript initializer. */
  return function<TThis, TArgs extends unknown[], TResult>(implementation: (this: TThis, ...args: TArgs) => Promise<TResult>, context: ClassMethodDecoratorContext<TThis, (this: TThis, ...args: TArgs) => Promise<TResult>>) {
    context.addInitializer(/** Registers local view bookkeeping on its owning service. */ function() {
      const receiver = this;
      ToolPkg.ipc.on<TArgs, TResult>(channel, async (args) => implementation.apply(receiver, args));
    });
    /** Invokes the view method in the owning main context. */
    return async function(this: TThis, ...args: TArgs): Promise<TResult> { return ToolPkg.ipc.call<TArgs, TResult>(channel, args, { targetRuntime: "main" }); };
  };
}

export class PlanModeShared {
  /** Reads the most recently active chat view from the owning runtime. */
  @Shared("plan_mode.get_single_active_chat_view")
  static async getSingleActiveChatView(): Promise<PlanModeTrackedChatView | null> {
    return readSingleActiveChatView();
  }

  /** Reads the tracked chat view identified by its chat id. */
  @Shared("plan_mode.get_tracked_chat_view_by_chat_id")
  static async getTrackedChatViewByChatId(chatId: string): Promise<PlanModeTrackedChatView | null> {
    return readTrackedChatViewByChatId(chatId);
  }

  /** Reads the planning flag for a chat from the owning runtime. */
  @ToolPkg.route({ binding: 0 })
  static async isEnabled(chatId: string): Promise<boolean> {
    return isPlanModeEnabledForChat(chatId);
  }

  /** Enables planning for a chat in the owning runtime. */
  @ToolPkg.route({ binding: 0 })
  static async enable(chatId: string): Promise<void> {
    await enablePlanModeForChat(chatId);
  }

  /** Disables planning for a chat in the owning runtime. */
  @ToolPkg.route({ binding: 0 })
  static async disable(chatId: string): Promise<void> {
    await disablePlanMode(chatId);
  }

  /** Reports whether a chat has a persisted plugin-owned plan. */
  @ToolPkg.route({ binding: 0 })
  static async hasPlanFile(chatId: string): Promise<boolean> {
    return await hasPlanFile(chatId);
  }

  /** Persists a chat plan in synchronized plugin storage. */
  @ToolPkg.route({ binding: 0 })
  static async writePlanFile(chatId: string, content: string): Promise<PlanFileRecord> {
    return await writePlanFile(chatId, content);
  }

  /** Reports whether this plan was already handed off for implementation. */
  @ToolPkg.route({ binding: 1 })
  static async isPlanStarted(planContent: string, chatId: string): Promise<boolean> {
    return await isPlanStarted(chatId, planContent);
  }

  /** Claims implementation handoff for a plan exactly once. */
  @ToolPkg.route({ binding: 1 })
  static async claimPlanStart(planContent: string, chatId: string): Promise<boolean> {
    return await claimPlanStart(chatId, planContent);
  }

  /** Clears the implementation-handoff record for a plan. */
  @ToolPkg.route({ binding: 1 })
  static async forgetPlanStart(planContent: string, chatId: string): Promise<void> {
    await forgetPlanStart(chatId, planContent);
  }

  /** Stores the latest tracked chat view in the shared runtime. */
  @Shared("plan_mode.upsert_tracked_chat_view")
  static async upsertTrackedChatView(view: PlanModeTrackedChatView): Promise<void> {
    await upsertTrackedChatViewAsync(view);
  }

  /** Removes a closed chat view from the shared runtime. */
  @Shared("plan_mode.remove_tracked_chat_view")
  static async removeTrackedChatView(runtime: PlanModeRuntime, viewId: string): Promise<void> {
    await removeTrackedChatViewAsync(runtime, viewId);
  }
  /** Streams the durable handoff status for the exact normalized plan. */
  @ToolPkg.route({ binding: 0 })
  static async *watchStarted(chatId: string, content: string, signal?: AbortSignal): AsyncGenerator<boolean> {
    if (!signal) throw new Error("Plan stream requires its collector cancellation signal");
    yield* watchPlanStarted(chatId, content, signal);
  }
  /** Streams the durable plan record for one chat. */
  @ToolPkg.route({ binding: 0 })
  static async *watchPlan(chatId: string, signal?: AbortSignal): AsyncGenerator<PlanFileRecord | null> {
    if (!signal) throw new Error("Plan stream requires its collector cancellation signal");
    yield* watchPlanFile(chatId, signal);
  }

}
