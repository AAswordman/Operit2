/** Public goal interface for dependent packages; goals belong to actual chat identities. */
export interface GoalRecord { objective: string; status: "active" | "paused" | "completed"; startedAt: number; updatedAt: number }
export function read(chatId: string): Promise<GoalRecord | null> {
  return ToolPkg.callDependency("com.operit.goal_mode_bundle", "goal.read", { chatId });
}
export function create(chatId: string, objective: string): Promise<GoalRecord> {
  return ToolPkg.callDependency("com.operit.goal_mode_bundle", "goal.create", { chatId, objective });
}
export function status(chatId: string, status: GoalRecord["status"]): Promise<GoalRecord | null> {
  return ToolPkg.callDependency("com.operit.goal_mode_bundle", "goal.status", { chatId, status });
}
