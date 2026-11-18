/* METADATA
{
    "api_version": "2.0.0",
 "name":"maintenance", "enabled_by_default":true,
 "display_name":{"zh":"项目维护","en":"Project maintenance"},
 "description":{"zh":"将用户反馈登记为后台维护工单，并查询实际进度。","en":"Record feedback and inspect background maintenance."},
 "tools":[
  {"name":"submit_feedback","description":"保存用户原始反馈并启动后台诊断，立即返回工单；不会阻塞当前对话。自动使用当前聊天在软件中绑定的工作区。","parameters":[{"name":"feedback","type":"string","required":true,"description":"原始反馈，保留用户的表达"}]},
  {"name":"bind_feedback_chat","description":"使用聊天已绑定的工作区收集反馈；后续用户消息自动聚合为工单。停用时设置 enabled=false。","parameters":[{"name":"enabled","type":"boolean","required":true,"description":"是否启用收集"},{"name":"chat_id","type":"string","required":false,"description":"目标聊天 ID；省略时使用当前聊天"}]},
  {"name":"list_tickets","description":"查询维护工单及当前状态。","parameters":[]},
  {"name":"inspect_ticket","description":"只读检查工单的项目占用、子代理记录及宿主对话状态；未知不代表任务已经停止。","parameters":[{"name":"id","type":"string","required":true,"description":"工单 ID"}]},
  {"name":"get_ticket","description":"获取工单的诊断、验收和真实错误。","parameters":[{"name":"id","type":"string","required":true,"description":"工单 ID"}]},
  {"name":"add_ticket_note","description":"向工单追加用户的补充信息。","parameters":[{"name":"id","type":"string","required":true,"description":"工单 ID"},{"name":"text","type":"string","required":true,"description":"补充信息"}]},
  {"name":"restart_ticket","description":"在核对未完成工作后重新开始一个失败或待补充的工单。","parameters":[{"name":"id","type":"string","required":true,"description":"工单 ID"}]}
 ]
}
*/
import { call, tickets } from "./client.js";
import type { Ticket } from "./model.js";
export async function submit_feedback(params: { feedback: string }): Promise<Ticket> {
  const parentChatId = getChatId();
  if (typeof parentChatId !== "string" || !parentChatId.trim()) throw new Error("Maintenance requires an actual parent chat");
  const ticket = await call<Ticket>({ action: "create", parentChatId, projectPath: "", source: "chat", feedback: params.feedback });
  return tickets.start(ticket.id);
}
export async function bind_feedback_chat(params: { enabled: boolean; chat_id?: string }): Promise<unknown> {
  const chatId = params.chat_id ?? getChatId();
  if (typeof chatId !== "string" || !chatId.trim()) throw new Error("Maintenance requires an actual chat");
  if (typeof params.enabled !== "boolean") throw new Error("enabled must be boolean");
  return call({ action: "bind_chat", chatId, enabled: params.enabled });
}
export async function list_tickets(): Promise<Ticket[]> { return tickets.list(); }
export async function inspect_ticket(params: { id: string }): Promise<unknown> { return tickets.inspect(params.id); }
export async function get_ticket(params: { id: string }): Promise<Ticket> { return tickets.get(params.id); }
export async function add_ticket_note(params: { id: string; text: string }): Promise<Ticket> { return tickets.note(params.id, params.text); }
export async function restart_ticket(params: { id: string }): Promise<Ticket> { return tickets.start(params.id); }
