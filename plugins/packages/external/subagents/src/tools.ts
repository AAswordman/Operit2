/* METADATA
{
  "name": "subagents",
  "enabled_by_default": true,
  "display_name": { "zh": "子代理", "en": "Subagents" },
  "description": {
    "zh": "把独立任务交给子代理并行执行。创建后返回 agent ID，使用 wait_agent 等待，超时可继续等待同一任务。子代理继承源聊天的执行配置，任务说明应当自包含。",
    "en": "Delegate self-contained tasks to independent agents. Spawn returns immediately; wait_agent observes the same task across 30-second waits. Child chats inherit source configuration, not its message history."
  },
  "tools": [
    {
      "name": "spawn_agent",
      "description": "创建并启动独立子代理，立即返回 agent ID。name 是短名称，task 应提供完整目标和必要背景。空间内最多同时运行 8 个子代理。",
      "parameters": [
        { "name": "name", "type": "string", "required": true, "description": "显示在输入框上方的简短任务名称" },
        { "name": "task", "type": "string", "required": true, "description": "自包含的任务指令和必要背景" }
      ]
    },
    {
      "name": "wait_agent",
      "description": "等待指定子代理的当前轮次，任一结束即返回。默认等待 30 秒；超时只结束本次等待，任务继续执行，可继续调用 wait_agent。返回真实状态、结果和错误。",
      "parameters": [
        { "name": "agent_ids", "type": "array", "required": true, "description": "1..16 个不重复的本聊天子代理 ID" },
        { "name": "timeout_ms", "type": "integer", "required": false, "description": "等待毫秒数，1..30000，默认 30000" }
      ]
    },
    {
      "name": "send_input",
      "description": "给子代理发送新指令。interrupt=true 会请求取消当前轮次，等待真实结束后发送；false 在仍运行时明确报错。已关闭的子代理不能再次发送。",
      "parameters": [
        { "name": "agent_id", "type": "string", "required": true, "description": "本聊天创建的子代理 ID" },
        { "name": "message", "type": "string", "required": true, "description": "新指令" },
        { "name": "interrupt", "type": "boolean", "required": false, "description": "是否打断当前轮次，默认 false" }
      ]
    },
    {
      "name": "close_agent",
      "description": "取消并关闭子代理及其后代，等待实际执行停止。保留会话与执行记录；结束的子代理也应显式关闭。",
      "parameters": [ { "name": "agent_id", "type": "string", "required": true, "description": "本聊天创建的子代理 ID" } ]
    },
    {
      "name": "list_agents",
      "description": "列出当前聊天的子代理、轮次、运行状态及最终结果。",
      "parameters": []
    }
  ]
}
*/
import { Agents } from "./client.js";
import { DEFAULT_WAIT_MS, requireText, type AgentSummary, type ParentSnapshot, type WaitResult } from "./model.js";

/** Captures the real tool origin before any IPC await; caller-authored parent IDs are never accepted. */
function parentChat(): string {
  return requireText(getChatId(), "authenticated chatId");
}

/** Starts a self-contained task with the actual host-selected participant identity. */
export async function spawn_agent(params: { name: string; task: string }): Promise<AgentSummary> {
  const parentChatId = parentChat();
  const participant = getCallerParticipantId();
  return Agents.spawn(parentChatId, params.name, params.task, participant === undefined ? null : requireText(participant, "authenticated participantId"));
}

/** Observes a bounded wait without replacing, resubmitting or cancelling the selected tasks. */
export async function wait_agent(params: { agent_ids: string[]; timeout_ms?: number }): Promise<WaitResult> {
  const { timeout_ms = DEFAULT_WAIT_MS } = params;
  return Agents.wait(parentChat(), params.agent_ids, timeout_ms);
}

/** Sends one follow-up with an explicit interruption policy. */
export async function send_input(params: { agent_id: string; message: string; interrupt?: boolean }): Promise<AgentSummary> {
  const { interrupt = false } = params;
  return Agents.send(parentChat(), params.agent_id, params.message, interrupt);
}

/** Closes only an agent owned by the actual calling conversation. */
export async function close_agent(params: { agent_id: string }): Promise<AgentSummary> {
  return Agents.close(parentChat(), params.agent_id);
}

/** Discovers children and terminal receipts without waiting for an active task. */
export async function list_agents(): Promise<ParentSnapshot> {
  return Agents.list(parentChat());
}
