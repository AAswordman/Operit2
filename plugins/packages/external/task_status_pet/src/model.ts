export const PACKAGE = "com.operit.task_status_pet";
export const CHANNEL = "task-status-pet";
export const PET_ROUTE = `toolpkg:${PACKAGE}:ui:pet`;
export const DETAILS_ROUTE = `toolpkg:${PACKAGE}:ui:details`;
export const SETTINGS_ROUTE = `toolpkg:${PACKAGE}:ui:settings`;
export type Mood = "working" | "thinking" | "approval" | "completed" | "idle" | "error";
export interface Settings { enabled: boolean; backgroundOnly: boolean; keepIdle: boolean; width: number; }
export const DEFAULTS: Settings = { enabled: true, backgroundOnly: true, keepIdle: true, width: 128 };
export const MIN_WIDTH = 64;
export const MAX_WIDTH = 256;
export interface Task { chatId: string; state: string; active: boolean; message: string; title?: string; round?: number; toolName?: string; }
export interface View { mood: Mood; chatId: string; message: string; count: number; approvals: ToolPkg.ToolPermissionRequest[]; tasks: Task[]; sequence: number; }
export type Request = { kind: "status" } | { kind: "save"; settings: Settings } | { kind: "show" } | { kind: "hide" }
  | { kind: "details" } | { kind: "closeDetails" } | { kind: "switchChat"; chatId: string }
  | { kind: "respond"; chatId: string; requestId: string; decision: ToolPkg.ToolPermissionDecision };
export interface Status { settings: Settings; scope: string; permissionGranted: boolean; view: View; presentation: string; }
export const LABELS: Record<Mood, string> = { working: "正在执行", thinking: "正在思考", approval: "等待批准", completed: "任务完成", idle: "空闲待命", error: "需要处理" };

/** Rejects invalid settings before changing storage or the host window. */
export function validateSettings(value: Settings): Settings {
  if (typeof value.enabled !== "boolean" || typeof value.backgroundOnly !== "boolean" || typeof value.keepIdle !== "boolean"
    || !Number.isFinite(value.width) || value.width < MIN_WIDTH || value.width > MAX_WIDTH) throw new Error(`设置无效：大小须为 ${MIN_WIDTH}–${MAX_WIDTH}，开关须为布尔值`);
  return { enabled: value.enabled, backgroundOnly: value.backgroundOnly, keepIdle: value.keepIdle, width: Math.round(value.width) };
}

/** Selects the actionable state from structured task and approval data. */
export function selectView(tasks: Task[], approvals: ToolPkg.ToolPermissionRequest[], completed: string | null, sequence: number): View {
  const active = tasks.filter(task => task.active);
  const errors = tasks.filter(task => task.state === "error");
  const request = approvals[0];
  const task = errors[0] ?? active[0];
  let mood: Mood = "idle", chatId = "", message = "随时待命";
  if (request) { mood = "approval"; chatId = request.chatId; message = `${approvals.length} 项待批准 · ${request.tool.name}`; }
  else if (task) {
    chatId = task.chatId;
    mood = task.state === "error" ? "error" : task.state === "connecting" || task.state === "processing" ? "thinking" : "working";
    message = task.message || LABELS[mood];
  } else if (completed !== null) { mood = "completed"; chatId = completed; message = "任务完成啦"; }
  return { mood, chatId, message, count: active.length, approvals, tasks, sequence };
}

/** Describes real work without treating tool invocations or animation frames as rounds. */
export function taskProgress(task: Task): string {
  const phase = task.toolName ? `执行 ${task.toolName}` : task.state === "completed" ? "已完成" : task.state === "connecting" ? "连接中"
    : task.state === "processing" ? "思考中" : task.state === "summarizing" ? "整理上下文" : task.state === "error" ? "需要处理" : "处理中";
  return task.round !== undefined && task.round > 0 ? `第 ${task.round} 轮 · ${phase}` : phase;
}

/** Supplies two compact lines for the selected task or its approval. */
export function petCaption(view: View): string {
  const task = view.tasks.find(task => task.chatId === view.chatId);
  const title = (task?.title || (view.chatId ? "当前任务" : LABELS[view.mood])).replace(/\s+/g, " ").trim();
  const progress = view.mood === "approval" ? `${task?.round ? `第 ${task.round} 轮 · ` : ""}待批准 · ${view.approvals[0].tool.name}`
    : task ? taskProgress(task) : view.message;
  return `${title}\n${progress}${view.count > 1 ? ` · 共 ${view.count} 个任务` : ""}`;
}
