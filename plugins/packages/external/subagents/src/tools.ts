/* METADATA
{
    "api_version": "2.0.0",
  "name": "subagents",
  "enabled_by_default": true,
  "display_name": { "zh": "子代理", "en": "Subagents" },
  "description": {
    "zh": "子代理支持并发：独立任务可连续创建多个代理，不必每创建一个就等待。先用 get_capacity 查询空间共享上限与空位，按空位分批创建，再用 wait_agent 等待任一完成；任务应自包含，子代理不继承聊天历史。",
    "en": "Subagents run concurrently: spawn independent tasks before waiting. Use get_capacity to check shared slots, dispatch within the limit, then wait for any completion. Children inherit configuration, not chat history; give each a self-contained task. Nested delegation can be disabled in settings."
  },
  "tools": [
    {
      "name": "spawn_agent",
      "description": "创建并启动独立子代理，立即返回 agent ID，不等待完成。独立任务可同时运行；先用 get_capacity 查看当前空间的并发上限和可用名额。达到上限时等待已有代理完成再创建。task 应自包含。",
      "parameters": [
        { "name": "name", "type": "string", "required": true, "description": "显示在输入框上方的简短任务名称" },
        { "name": "task", "type": "string", "required": true, "description": "自包含的任务指令和必要背景" }
      ]
    },
    {
      "name": "get_capacity",
      "description": "查询整个空间共享的子代理并发上限、正在运行或启动的数量及可用名额。可用名额只是查询时的快照，其他会话可能同时启动任务。",
      "parameters": []
    },
    {
      "name": "wait_agent",
      "description": "等待指定子代理的当前轮次，任一结束即返回。未指定超时时使用用户设置的默认等待时间；超时只结束本次等待，任务继续执行。",
      "parameters": [
        { "name": "agent_ids", "type": "array", "required": true, "description": "1..16 个不重复的本聊天子代理 ID" },
        { "name": "timeout_ms", "type": "integer", "required": false, "description": "等待毫秒数，1..30000；省略时使用设置页的默认值" }
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
import { requireText, type AgentSummary, type Capacity, type ParentSnapshot, type WaitResult } from "./model.js";

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

/** Exposes the same shared admission counters used by the manager. */
export async function get_capacity(): Promise<Capacity> { return Agents.capacity(); }

/** Observes a bounded wait without replacing, resubmitting or cancelling the selected tasks. */
export async function wait_agent(params: { agent_ids: string[]; timeout_ms?: number }): Promise<WaitResult> {
  const parentChatId = parentChat();
  const timeout_ms = params.timeout_ms === undefined ? (await Agents.settings()).defaultWaitMs : params.timeout_ms;
  return Agents.wait(parentChatId, params.agent_ids, timeout_ms);
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
