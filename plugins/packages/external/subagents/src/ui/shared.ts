import type { AgentStatus } from "../model.js";

export const CONVERSATION_ROUTE = "toolpkg:com.operit.subagents:ui:conversation";

export const labels: Record<AgentStatus, string> = {
  starting: "正在启动", running: "运行中", completed: "已完成", cancelled: "已取消",
  failed: "执行失败", closing: "正在关闭", closed: "已关闭", interrupted: "观察已中断",
};

export const statusIcons: Record<AgentStatus, string> = {
  starting: "hourglassEmpty", running: "playArrow", completed: "checkCircle",
  cancelled: "cancel", failed: "errorOutline", closing: "hourglassEmpty",
  closed: "close", interrupted: "pause",
};

