import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl";
import type { ToolPkg as ToolPkgTypes } from "../../../../../types/toolpkg";
import type { PresentationCancel } from "../presentation";
import type { GroupExecutionRequest, GroupExecutionStatus } from "./hooks";
import { assertObject, requireId } from "../validation";

/** Declares the package-owned presentation input for the execution control route. */
interface GroupControlPresentation { readonly requestId: string; readonly input: { readonly mode: "group-execution"; readonly chatId: string } }

/** Retains the current explicit read and prevents controls from targeting a replaced submission. */
interface ControlState { value: GroupExecutionStatus | null; busy: boolean; loaded: boolean; error: string; finished: boolean }
/** Owns a single modal's exact submission actions and explicit refresh operation. */
export interface GroupControl {
  /** Reads once on initial mount without automatically retrying a failed request. */
  load(): Promise<void>;
  /** Reads the current retained submission only on an explicit refresh gesture. */
  refresh(): Promise<void>;
  /** Controls the precise submission last successfully displayed by this modal. */
  act(action: "cancel" | "resume"): Promise<void>;
  /** Closes the presentation without cancelling or resuming group execution. */
  close(): PresentationCancel;
}

/** Requires a real presentation and the exact plugin-owned control input. */
function controlInput(value: GroupControlPresentation | null): { requestId: string; chatId: string } {
  assertObject(value, "group control presentation");
  const requestId = requireId(value.requestId, "group control requestId");
  assertObject(value.input, "group control input");
  if (value.input.mode !== "group-execution" || Object.keys(value.input).length !== 2) throw new Error("Group control requires mode and chatId");
  return { requestId, chatId: requireId(value.input.chatId, "group control chatId") };
}

/** Validates the package IPC result before displaying or enabling an exact-submission control. */
function checkedStatus(value: GroupExecutionStatus | null, chatId: string, submissionId: string | null): GroupExecutionStatus | null {
  if (value === null) {
    if (submissionId !== null) throw new Error("Exact group submission returned no status");
    return null;
  }
  assertObject(value, "group execution status");
  requireId(value.submissionId, "group status submissionId");
  if (value.chatId !== chatId || (submissionId !== null && value.submissionId !== submissionId)) throw new Error("Group status changed its requested identity");
  if (value.status !== "running" && value.status !== "settled" && value.status !== "failed") throw new Error("Unknown group execution status");
  if ((value.status === "settled") !== (value.outcome !== null)) throw new Error("Group status and settlement disagree");
  if (value.status === "failed" ? typeof value.error !== "string" : value.error !== null) throw new Error("Group failure status is malformed");
  if (value.outcome !== null && (value.outcome.chatId !== chatId || value.outcome.submissionId !== value.submissionId)) throw new Error("Group outcome changed its submission identity");
  return value;
}

/** Uses only the existing package IPC and authenticated main runtime, with no platform-specific host or repository. */
export function createGroupControl(requestId: string, chatId: string, publish: (state: ControlState) => void, assertOwner: () => void): GroupControl {
  let state: ControlState = { value: null, busy: false, loaded: false, error: "", finished: false }, loading: Promise<void> | null = null;
  /** Publishes only state produced by the actual requested operation. */
  function update(change: Partial<ControlState>): void { state = { ...state, ...change }; publish(state); }
  /** Rejects concurrent or stale modal actions before making an IPC call. */
  function idle(): void {
    assertOwner();
    if (state.finished || state.busy) throw new Error("Group control is closed or already processing an action");
  }
  /** Executes one explicit request and preserves its original rejection in the visible state. */
  async function request(action: "current" | "cancel" | "resume", submissionId: string | null): Promise<void> {
    idle(); update({ busy: true, error: "" });
    try {
      const payload: GroupExecutionRequest = action === "current" ? { action, chatId } : { action, chatId, submissionId: submissionId! };
      const value = await ToolPkg.ipc.call<typeof payload, GroupExecutionStatus | null>("character-memory.group-execution", payload, { targetRuntime: "main" });
      assertOwner(); update({ value: checkedStatus(value, chatId, submissionId), loaded: true });
    } catch (failure) { update({ error: String(failure) }); throw failure; }
    finally { update({ busy: false }); }
  }
  return {
    /** Retains the initial load Promise including its original rejection. */
    load(): Promise<void> { if (loading === null) loading = request("current", null); return loading; },
    /** Reads current state only when the user explicitly asks to refresh. */
    refresh(): Promise<void> { return request("current", null); },
    /** Requires the precise displayed state before issuing cancellation or resumption. */
    act(action): Promise<void> {
      idle(); const value = state.value;
      if (state.error !== "" || value === null) throw new Error("Group controls require a successful status read");
      if (action === "cancel" && value.status !== "running") throw new Error("Only a running submission can be cancelled");
      if (action === "resume" && (value.status !== "settled" || value.outcome === null || value.outcome.status !== "cancelled")) throw new Error("Only a settled cancelled submission can resume");
      return request(action, value.submissionId);
    },
    /** Emits cancellation of the modal only, not cancellation of the group submission. */
    close(): PresentationCancel { idle(); update({ finished: true }); return { type: "toolpkg.presentation.cancel", requestId }; },
  };
}

/** Renders a real registered Compose dialog with precise cancellation, resumption and explicit status refresh. */
export function renderGroupExecutionScreen(ctx: ComposeDslContext): ComposeNode {
  const [presentation] = ctx.useState<GroupControlPresentation | null>("presentation", null), input = controlInput(presentation);
  const [state, publish] = ctx.useState<ControlState>("group-control-state", { value: null, busy: false, loaded: false, error: "", finished: false });
  const owner = ctx.useRef("group-control-owner", JSON.stringify(presentation));
  const controller = ctx.useRef<GroupControl | null>("group-control-controller", null);
  /** Prevents retained callbacks from controlling another presentation's conversation. */
  function assertOwner(): void {
    const [current] = ctx.useState<GroupControlPresentation | null>("presentation", null);
    if (JSON.stringify(current) !== owner.current) throw new Error("Group control presentation owner changed");
  }
  assertOwner();
  if (controller.current === null) controller.current = createGroupControl(input.requestId, input.chatId, publish, assertOwner);
  const control = controller.current, value = state.value;
  const labels = { running: "正在执行", settled: "已结束", failed: "执行失败" };
  const outcomes = { completed: "已完成", cancelled: "已取消，可继续", blocked: "发送被阻止", consumed: "发送被其他处理器接管", not_persisted: "消息未保存" };
  const enabled = state.loaded && !state.busy && !state.finished && state.error === "";
  return ctx.UI.Dialog({ key: "group-execution-dialog", closeOnDismissRequest: false,
    properties: { dismissOnBackPress: !state.busy, dismissOnClickOutside: !state.busy },
    /** Loads the actual retained state once when this modal mounts. */
    onLoad: () => control.load(),
    /** Closes this modal without changing the underlying submission. */
    onDismissRequest: () => control.close(),
  }, ctx.UI.Column({ fillMaxWidth: true, paddingHorizontal: 20, paddingVertical: 16 }, [
    ctx.UI.Text({ text: "群组执行", style: "titleMedium" }),
    ctx.UI.Text({ key: "group-execution-status", text: !state.loaded ? "正在读取执行状态…" : value === null ? "当前会话没有已提交的群组任务" : labels[value.status], paddingVertical: 12 }),
    ...(value === null ? [] : [ctx.UI.Text({ text: "提交：" + value.submissionId }),
      ...(value.outcome === null ? [] : [ctx.UI.Text({ text: outcomes[value.outcome.status] + " · " + value.outcome.cursor + "/" + value.outcome.plannedTurns })]),
      ...(value.error === null ? [] : [ctx.UI.Text({ text: value.error, color: ctx.MaterialTheme.colorScheme.error })])]),
    ...(state.error === "" ? [] : [ctx.UI.Text({ key: "group-execution-error", text: state.error, color: ctx.MaterialTheme.colorScheme.error })]),
    ...(state.busy ? [ctx.UI.CircularProgressIndicator({ width: 20, height: 20 })] : []),
    ctx.UI.Row({ fillMaxWidth: true }, [
      ctx.UI.TextButton({ key: "group-execution-refresh", enabled: !state.busy && !state.finished,
        /** Requests a new status snapshot only for this explicit refresh click. */
        onClick: () => control.refresh(),
      }, ctx.UI.Text({ text: "刷新" })),
      ctx.UI.TextButton({ key: "group-execution-cancel", enabled: enabled && value !== null && value.status === "running",
        /** Cancels only the exact running submission currently displayed. */
        onClick: () => control.act("cancel"),
      }, ctx.UI.Text({ text: "取消执行" })),
      ctx.UI.TextButton({ key: "group-execution-resume", enabled: enabled && value !== null && value.status === "settled" && value.outcome !== null && value.outcome.status === "cancelled",
        /** Resumes only the exact cancelled submission without resending its user input. */
        onClick: () => control.act("resume"),
      }, ctx.UI.Text({ text: "继续执行" })),
    ]),
    ctx.UI.IconButton({ key: "group-execution-close", enabled: !state.busy && !state.finished,
      /** Dismisses this presentation and leaves execution unchanged. */
      onClick: () => control.close(),
    }, ctx.UI.Icon({ name: "Close", contentDescription: "关闭" })),
  ]));
}
